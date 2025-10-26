#![allow(unused_imports)]

#[macro_use] extern crate rocket;

use std::io::{Seek, SeekFrom, Write};
use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::panic;
use std::collections::HashMap;

use anyhow::{Context, Error};

use rocket::fs::{FileServer, TempFile};
use rocket::form::{Form, FromForm};
use rocket::serde::json::Json;
use rocket::serde::{Serialize, Deserialize};
use rocket::{get, post, launch, routes, State};
use rocket::response::stream::{Event, EventStream};
use rocket::tokio::select;
use rocket::tokio::time::{interval, Duration};
use rocket_dyn_templates::{Template, context};

use iso2god::executable::{TitleExecutionInfo, TitleInfo};
use iso2god::{game_list, god};
use iso2god::iso;
use iso2god::god::ContentType;

use rayon::prelude::*;
use rayon::iter::ParallelIterator;

use tempfile::tempdir;
use walkdir::WalkDir;
use suppaftp::FtpStream;
use suppaftp::native_tls::{TlsConnector, TlsStream};

#[derive(Clone, Serialize, Deserialize)]
struct FtpProgress {
    current_file: String,
    files_transferred: usize,
    total_files: usize,
    percentage: u8,
    message: String,
    is_complete: bool,
}

impl Default for FtpProgress {
    fn default() -> Self {
        Self {
            current_file: String::new(),
            files_transferred: 0,
            total_files: 0,
            percentage: 0,
            message: "Initializing...".to_string(),
            is_complete: false,
        }
    }
}

type FtpProgressMap = Arc<Mutex<HashMap<String, FtpProgress>>>;

#[derive(Serialize, Deserialize)]
struct IsoFile {
    path: String,
    name: String,
    size: u64,
}

#[derive(FromForm)]
struct ConversionForm<'f> {
    #[field(name = "source-iso")]
    source_iso: Option<TempFile<'f>>,
    #[field(name = "source-iso-path")]
    source_iso_path: Option<String>,
    #[field(name = "dest-dir")]
    dest_dir: String,
    #[field(name = "game-title")]
    game_title: Option<String>,
    #[field(name = "trim-mode")]
    trim_mode: String,
    #[field(name = "num-threads")]
    num_threads: String,
    #[field(name = "dry-run")]
    dry_run: bool,
}

#[derive(Serialize, Deserialize)]
struct ConversionResponse {
    success: bool,
    message: String,
    god_path: Option<String>,
}

#[derive(Serialize, Deserialize)]
struct FtpTransferRequest {
    session_id: String,
    god_path: String,
    ftp_host: String,
    ftp_port: u16,
    ftp_username: String,
    ftp_password: String,
    ftp_target_path: String,
}

#[derive(Serialize, Deserialize)]
struct FtpTransferResponse {
    success: bool,
    message: String,
    files_transferred: usize,
    session_id: String,
}

#[get("/")]
fn index() -> Template {
    Template::render("index", context! {})
}

#[get("/ftp-progress/<session_id>")]
fn ftp_progress(session_id: String, progress_map: &State<FtpProgressMap>) -> EventStream![] {
    let progress_map = progress_map.inner().clone();

    EventStream! {
        let mut interval = interval(Duration::from_millis(100));

        loop {
            interval.tick().await;

            let progress = {
                let map = progress_map.lock().unwrap();
                map.get(&session_id).cloned()
            };

            if let Some(prog) = progress {
                let is_complete = prog.is_complete;
                yield Event::json(&prog);

                if is_complete {
                    break;
                }
            } else {
                // Session not found or not started yet
                yield Event::json(&FtpProgress::default());
            }
        }
    }
}

#[derive(Serialize, Deserialize)]
struct ConvertedGame {
    path: String,
    name: String,
}

#[get("/list-converted-games")]
fn list_converted_games() -> Json<Vec<ConvertedGame>> {
    let output_dir = Path::new("/data/output");
    let mut games = Vec::new();

    if !output_dir.exists() {
        return Json(games);
    }

    // Scan output directory for GOD files (they're in format TitleID/ContentID/)
    for entry in fs::read_dir(output_dir).into_iter().flatten() {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };

        let title_path = entry.path();
        if title_path.is_dir() {
            // This is the title ID directory
            let title_id = title_path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("Unknown");

            games.push(ConvertedGame {
                path: title_path.to_string_lossy().to_string(),
                name: format!("Title ID: {}", title_id),
            });
        }
    }

    games.sort_by(|a, b| a.name.cmp(&b.name));
    Json(games)
}

#[get("/list-isos")]
fn list_isos() -> Json<Vec<IsoFile>> {
    let input_dir = Path::new("/data/input");
    let mut iso_files = Vec::new();

    if !input_dir.exists() {
        return Json(iso_files);
    }

    for entry in WalkDir::new(input_dir)
        .follow_links(true)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        if path.is_file() {
            if let Some(ext) = path.extension() {
                if ext.eq_ignore_ascii_case("iso") {
                    if let Ok(metadata) = fs::metadata(path) {
                        if let Some(name) = path.file_name() {
                            iso_files.push(IsoFile {
                                path: path.to_string_lossy().to_string(),
                                name: name.to_string_lossy().to_string(),
                                size: metadata.len(),
                            });
                        }
                    }
                }
            }
        }
    }

    iso_files.sort_by(|a, b| a.name.cmp(&b.name));
    Json(iso_files)
}

#[post("/convert", data = "<form>")]
async fn convert(mut form: Form<ConversionForm<'_>>) -> Json<ConversionResponse> {
    // Determine source ISO path: either from upload or from existing path
    let (source_iso_path, is_temp) = if let Some(iso_path) = &form.source_iso_path {
        // Use existing ISO from mounted directory
        if iso_path.is_empty() {
            return Json(ConversionResponse {
                success: false,
                message: "No ISO file path provided".to_string(),
                god_path: None,
            });
        }
        (PathBuf::from(iso_path), false)
    } else if let Some(ref mut uploaded_iso) = form.source_iso {
        // Handle uploaded ISO
        let temp_dir = match tempdir() {
            Ok(dir) => dir,
            Err(e) => return Json(ConversionResponse {
                success: false,
                message: e.to_string(),
                god_path: None,
            })
        };
        let mut temp_path = temp_dir.path().to_path_buf();
        temp_path.push("source.iso");

        if let Err(e) = uploaded_iso.copy_to(&temp_path).await {
            return Json(ConversionResponse {
                success: false,
                message: e.to_string(),
                god_path: None,
            });
        }
        (temp_path, true)
    } else {
        return Json(ConversionResponse {
            success: false,
            message: "No ISO file provided (neither upload nor path)".to_string(),
            god_path: None,
        });
    };

    let dest_dir_path = PathBuf::from(form.dest_dir.clone());
    let game_title = form.game_title.clone();
    let trim_mode = form.trim_mode.clone();

    // Parse num_threads - handle "auto" or numeric value
    let num_threads = if form.num_threads == "auto" {
        // Use available_parallelism from std (Rust 1.59+)
        std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(4) // Fallback to 4 if detection fails
    } else {
        form.num_threads.parse::<usize>().unwrap_or(1)
    };

    let dry_run = form.dry_run;

    let source_iso_path_for_cleanup = source_iso_path.clone();

    let result = tokio::task::spawn_blocking(move || {
        let result = panic::catch_unwind(move || {
            convert_iso(
                source_iso_path,
                dest_dir_path,
                game_title,
                trim_mode,
                num_threads,
                dry_run,
            )
        });

        // Only remove temp file if it was uploaded
        if is_temp {
            let _ = fs::remove_file(&source_iso_path_for_cleanup);
        }

        result
    }).await;

    match result {
        Ok(Ok(Ok((message, god_path)))) => Json(ConversionResponse {
            success: true,
            message,
            god_path: Some(god_path),
        }),
        Ok(Ok(Err(e))) => Json(ConversionResponse {
            success: false,
            message: e.to_string(),
            god_path: None,
        }),
        Ok(Err(_)) => Json(ConversionResponse {
            success: false,
            message: "Conversion process panicked".to_string(),
            god_path: None,
        }),
        Err(e) => Json(ConversionResponse {
            success: false,
            message: format!("Task execution failed: {}", e),
            god_path: None,
        }),
    }
}

fn convert_iso(
    source_iso: PathBuf,
    dest_dir: PathBuf,
    game_title: Option<String>,
    trim_mode: String,
    num_threads: usize,
    dry_run: bool,
) -> Result<(String, String), Error> {
    if num_threads == 1 {
        eprintln!(
            "The default number of threads was changed to 1 because of the problems witn Windows and/or hard drives."
        );
        eprintln!(
            "If you don't use Windows or use and SSD, might be worth increasing it with the -j <N> flag!"
        );
    }

    // Only initialize thread pool if not already initialized
    // Subsequent requests will reuse the existing pool
    let _ = rayon::ThreadPoolBuilder::new()
        .num_threads(num_threads)
        .build_global();

    let source_iso_file = File::open(&source_iso).context("error opening source ISO file")?;
    let source_iso_file_meta = fs::metadata(&source_iso).context("error reading source ISO file metadata")?;
    let mut source_iso_reader = iso::IsoReader::read(source_iso_file).context("error reading source ISO")?;

    let title_info = TitleInfo::from_image(&mut source_iso_reader).context("error reading image executable")?;
    let exe_info = title_info.execution_info;
    let content_type = title_info.content_type;

    let title_id_str = {
        let title_id = format!("{:08X}", exe_info.title_id);
        let name = game_list::find_title_by_id(exe_info.title_id).unwrap_or("(unknown)".to_owned());

        let mut result = String::new();
        result.push_str(&format!("Title ID: {}\n", title_id));
        result.push_str(&format!("    Name: {}\n", name));
        match content_type {
            ContentType::GamesOnDemand => result.push_str("    Type: Games on Demand\n"),
            ContentType::XboxOriginal => result.push_str("    Type: Xbox Original\n"),
        }
        result
    };

    if dry_run {
        // For dry run, return empty god_path since nothing was created
        return Ok((title_id_str, String::new()));
    }

    let data_size = if trim_mode == "from-end" {
        source_iso_reader.get_max_used_prefix_size()
    } else {
        let root_offset = source_iso_reader.volume_descriptor.root_offset;
        source_iso_file_meta.len() - root_offset
    };

    let block_count = data_size.div_ceil(god::BLOCK_SIZE);
    let part_count = block_count.div_ceil(god::BLOCKS_PER_PART);

    let file_layout = god::FileLayout::new(&dest_dir, &exe_info, content_type);

    ensure_empty_dir(&file_layout.data_dir_path()).context("error clearing data directory")?;

    let progress = AtomicUsize::new(0);

    (0..part_count).into_par_iter().try_for_each(|part_index| {
        let mut iso_data_volume = File::open(&source_iso)?;
        iso_data_volume.seek(SeekFrom::Start(source_iso_reader.volume_descriptor.root_offset))?;

        let part_file = file_layout.part_file_path(part_index);

        let part_file = File::options()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&part_file)
            .context("error creating part file")?;

        god::write_part(iso_data_volume, part_index, part_file)
            .context("error writing part file")?;

        let cur = 1 + progress.fetch_add(1, Ordering::Relaxed);
        eprintln!("writing part files: {cur:2}/{part_count}");

        Ok::<_, anyhow::Error>(())
    })?;

    let mut mht = read_part_mht(&file_layout, part_count - 1).context("error reading part file MHT")?;

    for prev_part_index in (0..part_count - 1).rev() {
        let mut prev_mht = read_part_mht(&file_layout, prev_part_index).context("error reading part file MHT")?;

        prev_mht.add_hash(&mht.digest());

        write_part_mht(&file_layout, prev_part_index, &prev_mht)
            .context("error writing part file MHT")?;

        mht = prev_mht;
    }

    let last_part_size = fs::metadata(file_layout.part_file_path(part_count - 1))
        .map(|m| m.len())
        .context("error reading part file")?;

    let mut con_header = god::ConHeaderBuilder::new()
        .with_execution_info(&exe_info)
        .with_block_counts(block_count as u32, 0)
        .with_data_parts_info(
            part_count as u32,
            last_part_size + (part_count - 1) * god::BLOCK_SIZE * 0xa290,
        )
        .with_content_type(content_type)
        .with_mht_hash(&mht.digest());

    let game_title_final = game_title.or(game_list::find_title_by_id(exe_info.title_id));
    if let Some(game_title) = game_title_final {
        con_header = con_header.with_game_title(&game_title);
    }

    let con_header = con_header.finalize();

    let mut con_header_file = File::options()
        .write(true)
        .create(true)
        .truncate(true)
        .open(file_layout.con_header_file_path())
        .context("cannot open con header file")?;

    con_header_file
        .write_all(&con_header)
        .context("error writing con header file")?;

    // The GOD path is the title directory (base_path/title_id)
    let god_path = file_layout.con_header_file_path()
        .parent()
        .and_then(|p| p.parent())
        .unwrap()
        .to_string_lossy()
        .to_string();

    Ok((format!("{}Conversion successful!", title_id_str), god_path))
}

fn ensure_empty_dir(path: &Path) -> Result<(), Error> {
    if fs::exists(path)? {
        fs::remove_dir_all(path)?;
    };
    fs::create_dir_all(path)?;
    Ok(())
}

fn read_part_mht(file_layout: &god::FileLayout, part_index: u64) -> Result<god::HashList, Error> {
    let part_file = file_layout.part_file_path(part_index);
    let mut part_file = File::options().read(true).open(part_file)?;
    god::HashList::read(&mut part_file)
}

fn write_part_mht(
    file_layout: &god::FileLayout,
    part_index: u64,
    mht: &god::HashList,
) -> Result<(), Error> {
    let part_file = file_layout.part_file_path(part_index);
    let mut part_file = File::options().write(true).open(part_file)?;
    mht.write(&mut part_file)?;
    Ok(())
}

#[post("/ftp-transfer", format = "json", data = "<request>")]
async fn ftp_transfer(
    request: Json<FtpTransferRequest>,
    progress_map: &State<FtpProgressMap>,
) -> Json<FtpTransferResponse> {
    let god_path = PathBuf::from(&request.god_path);
    let ftp_host = request.ftp_host.clone();
    let ftp_port = request.ftp_port;
    let ftp_username = request.ftp_username.clone();
    let ftp_password = request.ftp_password.clone();
    let ftp_target_path = request.ftp_target_path.clone();
    let session_id = request.session_id.clone();
    let session_id_for_response = session_id.clone();

    let progress_map_clone = progress_map.inner().clone();

    let result = tokio::task::spawn_blocking(move || {
        transfer_to_ftp(
            &god_path,
            &ftp_host,
            ftp_port,
            &ftp_username,
            &ftp_password,
            &ftp_target_path,
            session_id,
            progress_map_clone,
        )
    })
    .await;

    match result {
        Ok(Ok(count)) => Json(FtpTransferResponse {
            success: true,
            message: format!("Successfully transferred {} files to Xbox 360", count),
            files_transferred: count,
            session_id: session_id_for_response,
        }),
        Ok(Err(e)) => Json(FtpTransferResponse {
            success: false,
            message: format!("FTP transfer failed: {}", e),
            files_transferred: 0,
            session_id: session_id_for_response,
        }),
        Err(e) => Json(FtpTransferResponse {
            success: false,
            message: format!("Task execution failed: {}", e),
            files_transferred: 0,
            session_id: session_id_for_response,
        }),
    }
}

fn transfer_to_ftp(
    god_path: &Path,
    ftp_host: &str,
    ftp_port: u16,
    username: &str,
    password: &str,
    target_path: &str,
    session_id: String,
    progress_map: FtpProgressMap,
) -> Result<usize, Error> {
    // Helper to update progress
    let update_progress = |progress: FtpProgress| {
        let mut map = progress_map.lock().unwrap();
        map.insert(session_id.clone(), progress);
    };

    update_progress(FtpProgress {
        message: format!("Connecting to FTP server {}:{}", ftp_host, ftp_port),
        ..Default::default()
    });

    eprintln!("Connecting to FTP server {}:{}", ftp_host, ftp_port);

    // Connect to FTP server
    let mut ftp_stream = FtpStream::connect(format!("{}:{}", ftp_host, ftp_port))
        .context("Failed to connect to FTP server")?;

    // Login
    ftp_stream
        .login(username, password)
        .context("FTP login failed")?;

    update_progress(FtpProgress {
        message: "FTP login successful".to_string(),
        ..Default::default()
    });

    eprintln!("FTP login successful");

    // Set binary mode (important for GOD files!)
    ftp_stream.transfer_type(suppaftp::types::FileType::Binary)
        .context("Failed to set binary transfer mode")?;

    // Create target directory if needed
    let _ = ftp_stream.mkdir(target_path);
    ftp_stream.cwd(target_path)
        .context(format!("Failed to change to target directory: {}", target_path))?;

    // Count total files first
    let total_files = WalkDir::new(god_path)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_file())
        .count();

    update_progress(FtpProgress {
        total_files,
        message: format!("Starting transfer of {} files", total_files),
        ..Default::default()
    });

    let mut files_transferred = 0;

    // Walk through the GOD directory structure
    for entry in WalkDir::new(god_path).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();

        if path.is_file() {
            // Get relative path from god_path
            let relative_path = path.strip_prefix(god_path)?;
            let remote_path = relative_path.to_string_lossy().to_string();

            update_progress(FtpProgress {
                current_file: remote_path.clone(),
                files_transferred,
                total_files,
                percentage: if total_files > 0 {
                    ((files_transferred as f64 / total_files as f64) * 100.0) as u8
                } else {
                    0
                },
                message: format!("Uploading: {}", remote_path),
                is_complete: false,
            });

            eprintln!("Uploading: {}", remote_path);

            // Create parent directories on FTP server
            if let Some(parent) = relative_path.parent() {
                let parent_str = parent.to_string_lossy();
                if !parent_str.is_empty() {
                    for component in parent.components() {
                        let dir_name = component.as_os_str().to_string_lossy();
                        let _ = ftp_stream.mkdir(&dir_name);
                        ftp_stream.cwd(&dir_name)?;
                    }
                    // Go back to target root
                    ftp_stream.cwd(target_path)?;
                }
            }

            // Upload the file
            let mut file = File::open(path)
                .context(format!("Failed to open file: {:?}", path))?;

            ftp_stream
                .put_file(&remote_path, &mut file)
                .context(format!("Failed to upload file: {}", remote_path))?;

            files_transferred += 1;
            eprintln!("Uploaded: {} ({} files total)", remote_path, files_transferred);
        }
    }

    // Logout and close connection
    ftp_stream.quit()
        .context("Failed to disconnect from FTP server")?;

    update_progress(FtpProgress {
        current_file: String::new(),
        files_transferred,
        total_files,
        percentage: 100,
        message: format!("FTP transfer complete: {} files transferred", files_transferred),
        is_complete: true,
    });

    eprintln!("FTP transfer complete: {} files transferred", files_transferred);
    Ok(files_transferred)
}

#[launch]
fn rocket() -> rocket::Rocket<rocket::Build> {
    let progress_map: FtpProgressMap = Arc::new(Mutex::new(HashMap::new()));

    rocket::build()
        .manage(progress_map)
        .mount("/", routes![index, list_isos, list_converted_games, convert, ftp_transfer, ftp_progress])
        .mount("/public", FileServer::from("public"))
        .attach(Template::fairing())
}
