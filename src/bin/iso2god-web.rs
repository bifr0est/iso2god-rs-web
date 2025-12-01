use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{Seek, SeekFrom, Write};
use std::panic;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use anyhow::{Context, Error};

use rocket::form::{Form, FromForm};
use rocket::fs::{FileServer, TempFile};
use rocket::response::stream::{Event, EventStream};
use rocket::serde::json::Json;
use rocket::serde::{Deserialize, Serialize};
use rocket::tokio::time::{interval, Duration};
use rocket::{get, launch, post, routes, State};
use rocket_dyn_templates::{context, Template};

use iso2god::executable::TitleInfo;
use iso2god::god::ContentType;
use iso2god::iso;
use iso2god::{game_list, god};

use rayon::prelude::*;

use suppaftp::FtpStream;
use tempfile::tempdir;
use walkdir::WalkDir;

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
    game_title: Option<String>,
    title_id: Option<String>,
}

#[derive(Serialize, Deserialize)]
struct IsoInfoResponse {
    success: bool,
    game_title: Option<String>,
    title_id: Option<String>,
    error: Option<String>,
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
    #[serde(default)]
    passive_mode: bool,
}

#[derive(Serialize, Deserialize)]
struct FtpTestRequest {
    ftp_host: String,
    ftp_port: u16,
    ftp_username: String,
    ftp_password: String,
    #[serde(default)]
    passive_mode: bool,
}

#[derive(Serialize, Deserialize)]
struct FtpTestResponse {
    success: bool,
    message: String,
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
                        // Use relative path from /data/input for better display
                        let display_name = path.strip_prefix(input_dir)
                            .unwrap_or(path)
                            .to_string_lossy()
                            .to_string();

                        iso_files.push(IsoFile {
                            path: path.to_string_lossy().to_string(),
                            name: display_name,
                            size: metadata.len(),
                        });
                    }
                }
            }
        }
    }

    iso_files.sort_by(|a, b| a.name.cmp(&b.name));
    Json(iso_files)
}

#[get("/iso-info?<path>")]
fn get_iso_info(path: String) -> Json<IsoInfoResponse> {
    match get_iso_title_info(&path) {
        Ok((title, id)) => Json(IsoInfoResponse {
            success: true,
            game_title: Some(title),
            title_id: Some(id),
            error: None,
        }),
        Err(e) => Json(IsoInfoResponse {
            success: false,
            game_title: None,
            title_id: None,
            error: Some(e.to_string()),
        }),
    }
}

fn get_iso_title_info(iso_path: &str) -> Result<(String, String), Error> {
    let source_iso_file = File::open(iso_path).context("error opening source ISO file")?;
    let mut source_iso_reader = iso::IsoReader::read(source_iso_file).context("error reading source ISO")?;
    let title_info = TitleInfo::from_image(&mut source_iso_reader).context("error reading image executable")?;
    let exe_info = title_info.execution_info;

    let title_id = format!("{:08X}", exe_info.title_id);
    let game_name = game_list::find_title_by_id(exe_info.title_id).unwrap_or("(unknown)".to_owned());

    Ok((game_name, title_id))
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
                game_title: None,
                title_id: None,
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
                game_title: None,
                title_id: None,
            })
        };
        let mut temp_path = temp_dir.path().to_path_buf();
        temp_path.push("source.iso");

        if let Err(e) = uploaded_iso.copy_to(&temp_path).await {
            return Json(ConversionResponse {
                success: false,
                message: e.to_string(),
                god_path: None,
                game_title: None,
                title_id: None,
            });
        }
        (temp_path, true)
    } else {
        return Json(ConversionResponse {
            success: false,
            message: "No ISO file provided (neither upload nor path)".to_string(),
            god_path: None,
            game_title: None,
            title_id: None,
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
        Ok(Ok(Ok((message, god_path, game_title, title_id)))) => Json(ConversionResponse {
            success: true,
            message,
            god_path: Some(god_path),
            game_title: Some(game_title),
            title_id: Some(title_id),
        }),
        Ok(Ok(Err(e))) => Json(ConversionResponse {
            success: false,
            message: e.to_string(),
            god_path: None,
            game_title: None,
            title_id: None,
        }),
        Ok(Err(_)) => Json(ConversionResponse {
            success: false,
            message: "Conversion process panicked".to_string(),
            god_path: None,
            game_title: None,
            title_id: None,
        }),
        Err(e) => Json(ConversionResponse {
            success: false,
            message: format!("Task execution failed: {}", e),
            god_path: None,
            game_title: None,
            title_id: None,
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
) -> Result<(String, String, String, String), Error> {
    // Try to initialize global thread pool, but don't fail if already initialized
    // The first request sets the pool size; subsequent requests reuse it
    match rayon::ThreadPoolBuilder::new()
        .num_threads(num_threads)
        .build_global()
    {
        Ok(_) => eprintln!("Thread pool initialized with {} threads", num_threads),
        Err(_) => eprintln!("Using existing thread pool (requested {} threads)", num_threads),
    }

    let source_iso_file = File::open(&source_iso).context("error opening source ISO file")?;
    let source_iso_file_meta = fs::metadata(&source_iso).context("error reading source ISO file metadata")?;
    let mut source_iso_reader = iso::IsoReader::read(source_iso_file).context("error reading source ISO")?;

    let title_info = TitleInfo::from_image(&mut source_iso_reader).context("error reading image executable")?;
    let exe_info = title_info.execution_info;
    let content_type = title_info.content_type;

    let title_id = format!("{:08X}", exe_info.title_id);
    let game_name = game_list::find_title_by_id(exe_info.title_id).unwrap_or("(unknown)".to_owned());

    let title_id_str = {
        let mut result = String::new();
        result.push_str(&format!("Title ID: {}\n", title_id));
        result.push_str(&format!("    Name: {}\n", game_name));
        match content_type {
            ContentType::GamesOnDemand => result.push_str("    Type: Games on Demand\n"),
            ContentType::XboxOriginal => result.push_str("    Type: Xbox Original\n"),
        }
        result
    };

    if dry_run {
        // For dry run, return empty god_path since nothing was created
        return Ok((title_id_str, String::new(), game_name, title_id));
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

    Ok((format!("{}Conversion successful!", title_id_str), god_path, game_name, title_id))
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

/// Test FTP connection without transferring any files
#[post("/ftp-test", format = "json", data = "<request>")]
async fn ftp_test(request: Json<FtpTestRequest>) -> Json<FtpTestResponse> {
    let ftp_host = request.ftp_host.clone();
    let ftp_port = request.ftp_port;
    let ftp_username = request.ftp_username.clone();
    let ftp_password = request.ftp_password.clone();
    let passive_mode = request.passive_mode;

    let result = tokio::task::spawn_blocking(move || {
        test_ftp_connection(&ftp_host, ftp_port, &ftp_username, &ftp_password, passive_mode)
    })
    .await;

    match result {
        Ok(Ok(msg)) => Json(FtpTestResponse {
            success: true,
            message: msg,
        }),
        Ok(Err(e)) => Json(FtpTestResponse {
            success: false,
            message: format!("Connection failed: {}", e),
        }),
        Err(e) => Json(FtpTestResponse {
            success: false,
            message: format!("Task failed: {}", e),
        }),
    }
}

fn test_ftp_connection(
    ftp_host: &str,
    ftp_port: u16,
    username: &str,
    password: &str,
    passive_mode: bool,
) -> Result<String, Error> {
    // Connect with timeout
    let mut ftp_stream = FtpStream::connect_timeout(
        format!("{}:{}", ftp_host, ftp_port)
            .parse()
            .map_err(|e| anyhow::anyhow!("Invalid address: {}", e))?,
        Duration::from_secs(10),
    )
    .context("Failed to connect to FTP server")?;

    // Set passive mode if requested
    if passive_mode {
        ftp_stream.set_passive_nat_workaround(true);
    }

    // Login
    ftp_stream
        .login(username, password)
        .context("Login failed - check username and password")?;

    // Try to get current directory to verify connection works
    let pwd = ftp_stream.pwd().unwrap_or_else(|_| "/".to_string());

    // Disconnect
    let _ = ftp_stream.quit();

    Ok(format!(
        "Connection successful! Current directory: {}",
        pwd
    ))
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
    let passive_mode = request.passive_mode;

    let progress_map_clone = progress_map.inner().clone();

    let result = tokio::task::spawn_blocking(move || {
        transfer_to_ftp(
            &god_path,
            &ftp_host,
            ftp_port,
            &ftp_username,
            &ftp_password,
            &ftp_target_path,
            passive_mode,
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
    passive_mode: bool,
    session_id: String,
    progress_map: FtpProgressMap,
) -> Result<usize, Error> {
    // Helper to update progress
    let update_progress = |progress: FtpProgress| {
        let mut map = progress_map.lock().unwrap();
        map.insert(session_id.clone(), progress);
    };

    // Helper to cleanup session from progress map
    let cleanup_session = || {
        // Schedule cleanup after a delay to allow final progress read
        std::thread::spawn({
            let progress_map = progress_map.clone();
            let session_id = session_id.clone();
            move || {
                std::thread::sleep(Duration::from_secs(30));
                let mut map = progress_map.lock().unwrap();
                map.remove(&session_id);
                eprintln!("Cleaned up FTP session: {}", session_id);
            }
        });
    };

    let mode_str = if passive_mode { "passive" } else { "active" };
    update_progress(FtpProgress {
        message: format!("Connecting to FTP server {}:{} ({})", ftp_host, ftp_port, mode_str),
        ..Default::default()
    });

    eprintln!("Connecting to FTP server {}:{} ({})", ftp_host, ftp_port, mode_str);

    // Connect to FTP server with timeout
    let mut ftp_stream = FtpStream::connect_timeout(
        format!("{}:{}", ftp_host, ftp_port).parse().map_err(|e| anyhow::anyhow!("Invalid address: {}", e))?,
        Duration::from_secs(30)
    ).context("Failed to connect to FTP server (timeout: 30s)")?;

    // Set passive mode if requested (better for NAT/firewall)
    if passive_mode {
        ftp_stream.set_passive_nat_workaround(true);
    }

    // Login
    ftp_stream
        .login(username, password)
        .context("FTP login failed - check username and password")?;

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
    
    // Track created directories to avoid redundant mkdir calls
    let mut created_dirs: std::collections::HashSet<String> = std::collections::HashSet::new();

    // Walk through the GOD directory structure
    for entry in WalkDir::new(god_path).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();

        if path.is_file() {
            // Get relative path from god_path
            let relative_path = path.strip_prefix(god_path)?;
            let remote_path = relative_path.to_string_lossy().replace("\\", "/");

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

            // Create parent directories on FTP server (building full path incrementally)
            if let Some(parent) = relative_path.parent() {
                let parent_str = parent.to_string_lossy().replace("\\", "/");
                if !parent_str.is_empty() && !created_dirs.contains(&parent_str) {
                    // Build path incrementally: /target/dir1/dir2/...
                    let mut current_path = target_path.to_string();
                    for component in parent.components() {
                        let dir_name = component.as_os_str().to_string_lossy();
                        if current_path.ends_with('/') {
                            current_path = format!("{}{}", current_path, dir_name);
                        } else {
                            current_path = format!("{}/{}", current_path, dir_name);
                        }
                        // Only create if not already created
                        if !created_dirs.contains(&current_path) {
                            let _ = ftp_stream.mkdir(&current_path);
                            created_dirs.insert(current_path.clone());
                        }
                    }
                    created_dirs.insert(parent_str);
                }
            }

            // Build full remote path
            let full_remote_path = if target_path.ends_with('/') {
                format!("{}{}", target_path, remote_path)
            } else {
                format!("{}/{}", target_path, remote_path)
            };

            // Upload the file using absolute path
            let mut file = File::open(path)
                .context(format!("Failed to open file: {:?}", path))?;

            ftp_stream
                .put_file(&full_remote_path, &mut file)
                .context(format!("Failed to upload file: {}", full_remote_path))?;

            files_transferred += 1;
            eprintln!("Uploaded: {} ({}/{})", full_remote_path, files_transferred, total_files);
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

    // Schedule cleanup of this session from progress map
    cleanup_session();

    eprintln!("FTP transfer complete: {} files transferred", files_transferred);
    Ok(files_transferred)
}

#[launch]
fn rocket() -> rocket::Rocket<rocket::Build> {
    let progress_map: FtpProgressMap = Arc::new(Mutex::new(HashMap::new()));

    rocket::build()
        .manage(progress_map)
        .mount("/", routes![index, list_isos, list_converted_games, get_iso_info, convert, ftp_test, ftp_transfer, ftp_progress])
        .mount("/public", FileServer::from("public"))
        .attach(Template::fairing())
}
