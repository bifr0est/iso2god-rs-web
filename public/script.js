// Load ISO files on page load
async function loadIsoFiles() {
    try {
        const response = await fetch('/list-isos');
        const isos = await response.json();
        const select = document.getElementById('source-iso-path');

        if (isos.length === 0) {
            select.innerHTML = '<option value="" disabled>No ISO files found in /data/input</option>';
        } else {
            select.innerHTML = '';
            isos.forEach(iso => {
                const option = document.createElement('option');
                option.value = iso.path;
                const sizeMB = (iso.size / (1024 * 1024)).toFixed(2);
                option.textContent = `${iso.name} (${sizeMB} MB)`;
                select.appendChild(option);
            });
        }
    } catch (error) {
        console.error('Failed to load ISO files:', error);
        const select = document.getElementById('source-iso-path');
        select.innerHTML = '<option value="" disabled>Error loading ISO files</option>';
    }
}

// Auto-fill game title when ISO is selected (single selection)
document.getElementById('source-iso-path').addEventListener('change', async (event) => {
    const selectedOptions = Array.from(event.target.selectedOptions);
    const gameTitleField = document.getElementById('game-title');

    // Only auto-fill if single selection
    if (selectedOptions.length !== 1) {
        gameTitleField.value = '';
        gameTitleField.placeholder = selectedOptions.length > 1 ? 'Multiple files selected' : '';
        return;
    }

    const isoPath = selectedOptions[0].value;
    if (!isoPath) {
        gameTitleField.value = '';
        gameTitleField.placeholder = '';
        return;
    }

    // Show loading state
    gameTitleField.placeholder = 'Loading game info...';

    try {
        const response = await fetch(`/iso-info?path=${encodeURIComponent(isoPath)}`);
        const info = await response.json();

        if (info.success && info.game_title) {
            gameTitleField.value = info.game_title;
            gameTitleField.placeholder = info.game_title;
        } else {
            gameTitleField.value = '';
            gameTitleField.placeholder = 'Could not detect game title';
        }
    } catch (error) {
        console.error('Failed to get ISO info:', error);
        gameTitleField.value = '';
        gameTitleField.placeholder = 'Error loading game info';
    }
});

// Drag and Drop functionality
let uploadedFiles = [];

const dropZone = document.getElementById('drop-zone');
const fileInput = document.getElementById('source-iso');
const fileListContainer = document.getElementById('upload-file-list');

if (dropZone) {
    // Prevent default drag behaviors
    ['dragenter', 'dragover', 'dragleave', 'drop'].forEach(eventName => {
        dropZone.addEventListener(eventName, preventDefaults, false);
        document.body.addEventListener(eventName, preventDefaults, false);
    });

    function preventDefaults(e) {
        e.preventDefault();
        e.stopPropagation();
    }

    // Highlight drop zone when item is dragged over
    ['dragenter', 'dragover'].forEach(eventName => {
        dropZone.addEventListener(eventName, () => dropZone.classList.add('drag-over'), false);
    });

    ['dragleave', 'drop'].forEach(eventName => {
        dropZone.addEventListener(eventName, () => dropZone.classList.remove('drag-over'), false);
    });

    // Handle dropped files
    dropZone.addEventListener('drop', handleDrop, false);

    // Handle file input change
    fileInput.addEventListener('change', handleFileSelect, false);

    // Click on drop zone triggers file input
    dropZone.addEventListener('click', (e) => {
        if (e.target !== fileInput) {
            fileInput.click();
        }
    });
}

function handleDrop(e) {
    const dt = e.dataTransfer;
    const files = [...dt.files].filter(f => f.name.toLowerCase().endsWith('.iso'));
    addFilesToList(files);
}

function handleFileSelect(e) {
    const files = [...e.target.files].filter(f => f.name.toLowerCase().endsWith('.iso'));
    addFilesToList(files);
}

function addFilesToList(files) {
    files.forEach(file => {
        // Avoid duplicates
        if (!uploadedFiles.some(f => f.name === file.name && f.size === file.size)) {
            uploadedFiles.push(file);
        }
    });
    renderFileList();
}

function removeFile(index) {
    uploadedFiles.splice(index, 1);
    renderFileList();
}

function renderFileList() {
    if (!fileListContainer) return;
    
    if (uploadedFiles.length === 0) {
        fileListContainer.innerHTML = '';
        return;
    }

    fileListContainer.innerHTML = uploadedFiles.map((file, index) => {
        const sizeMB = (file.size / (1024 * 1024)).toFixed(2);
        return `
            <div class="upload-file-item">
                <span class="file-name">📀 ${file.name}</span>
                <span class="file-size">${sizeMB} MB</span>
                <button type="button" class="remove-file" onclick="removeFile(${index})">✕ Remove</button>
            </div>
        `;
    }).join('');
}

// Toggle between select and upload modes
document.querySelectorAll('input[name="source-type"]').forEach(radio => {
    radio.addEventListener('change', (event) => {
        const selectGroup = document.getElementById('select-iso-group');
        const uploadGroup = document.getElementById('upload-iso-group');

        if (event.target.value === 'select') {
            selectGroup.style.display = 'block';
            uploadGroup.style.display = 'none';
            document.getElementById('source-iso').removeAttribute('required');
        } else {
            selectGroup.style.display = 'none';
            uploadGroup.style.display = 'block';
        }
    });
});

// Conversion history management
let conversionHistory = JSON.parse(localStorage.getItem('conversionHistory') || '[]');

function addToHistory(item) {
    conversionHistory.unshift({
        ...item,
        timestamp: new Date().toISOString()
    });
    // Keep only last 10 items
    if (conversionHistory.length > 10) {
        conversionHistory = conversionHistory.slice(0, 10);
    }
    localStorage.setItem('conversionHistory', JSON.stringify(conversionHistory));
    displayHistory();
}

function displayHistory() {
    const historyList = document.getElementById('history-list');

    if (conversionHistory.length === 0) {
        historyList.innerHTML = '<p style="text-align: center; color: #999;">No conversions yet</p>';
        return;
    }

    historyList.innerHTML = conversionHistory.map(item => {
        const date = new Date(item.timestamp);
        const timeStr = date.toLocaleString();
        const statusClass = item.success ? 'success' : 'error';
        const statusText = item.success ? '✓ Success' : '✗ Failed';

        return `
            <div class="history-item ${statusClass}">
                <div class="history-item-info">
                    <div class="history-item-name">${item.name}</div>
                    <div class="history-item-time">${timeStr}</div>
                </div>
                <div class="history-item-status">${statusText}</div>
            </div>
        `;
    }).join('');
}

// Progress simulation (since we can't get real-time progress from the backend yet)
function simulateProgress(estimatedTime = 180000) { // 3 minutes default
    const progressContainer = document.getElementById('progress-container');
    const progressFill = document.getElementById('progress-fill');
    const progressText = document.getElementById('progress-text');
    const progressMessage = document.getElementById('progress-message');

    progressContainer.style.display = 'block';

    const stages = [
        { percent: 10, message: 'Reading ISO metadata...' },
        { percent: 20, message: 'Analyzing file structure...' },
        { percent: 40, message: 'Writing part files...' },
        { percent: 70, message: 'Calculating MHT hash chain...' },
        { percent: 90, message: 'Writing GOD header...' },
        { percent: 95, message: 'Finalizing...' }
    ];

    let currentStage = 0;
    const interval = estimatedTime / 100;
    let progress = 0;

    const timer = setInterval(() => {
        progress += 1;
        progressFill.style.width = progress + '%';
        progressText.textContent = progress + '%';

        // Update message based on stage
        if (currentStage < stages.length && progress >= stages[currentStage].percent) {
            progressMessage.textContent = stages[currentStage].message;
            currentStage++;
        }

        if (progress >= 95) {
            clearInterval(timer);
        }
    }, interval);

    return timer;
}

function resetProgress() {
    const progressContainer = document.getElementById('progress-container');
    const progressFill = document.getElementById('progress-fill');
    const progressText = document.getElementById('progress-text');

    progressContainer.style.display = 'none';
    progressFill.style.width = '0%';
    progressText.textContent = '0%';
}

function completeProgress() {
    const progressFill = document.getElementById('progress-fill');
    const progressText = document.getElementById('progress-text');
    const progressMessage = document.getElementById('progress-message');

    progressFill.style.width = '100%';
    progressText.textContent = '100%';
    progressMessage.textContent = 'Conversion complete!';

    setTimeout(() => {
        resetProgress();
    }, 2000);
}

// Handle form submission
document.getElementById('conversion-form').addEventListener('submit', async (event) => {
    event.preventDefault();

    const form = event.target;
    const statusDiv = document.getElementById('status');
    const convertBtn = document.getElementById('convert-btn');
    const sourceType = document.querySelector('input[name="source-type"]:checked').value;
    const autoTransfer = document.getElementById('auto-transfer').checked;

    // Collect files to convert
    let filesToConvert = [];
    
    if (sourceType === 'select') {
        const selectElement = document.getElementById('source-iso-path');
        const selectedOptions = Array.from(selectElement.selectedOptions);
        filesToConvert = selectedOptions.map(opt => ({
            type: 'path',
            path: opt.value,
            name: opt.text.split(' (')[0]
        })).filter(f => f.path);
    } else {
        // Use uploaded files from drag-and-drop or file input
        if (uploadedFiles.length > 0) {
            filesToConvert = uploadedFiles.map(file => ({
                type: 'upload',
                file: file,
                name: file.name
            }));
        }
    }

    if (filesToConvert.length === 0) {
        statusDiv.innerHTML = '⚠️ Please select at least one ISO file';
        statusDiv.className = 'status-error';
        statusDiv.style.display = 'block';
        return;
    }

    // Single file or batch?
    if (filesToConvert.length === 1) {
        await convertSingleFile(filesToConvert[0], form, statusDiv, convertBtn, autoTransfer);
    } else {
        await convertBatch(filesToConvert, form, statusDiv, convertBtn, autoTransfer);
    }
});

// Convert a single file
async function convertSingleFile(fileInfo, form, statusDiv, convertBtn, autoTransfer) {
    const formData = new FormData(form);
    
    if (fileInfo.type === 'path') {
        formData.set('source-iso-path', fileInfo.path);
        formData.delete('source-iso');
    } else {
        formData.set('source-iso', fileInfo.file);
        formData.delete('source-iso-path');
    }

    const originalButtonText = autoTransfer ? 'Convert & Transfer' : 'Convert';

    // Disable form
    convertBtn.disabled = true;
    convertBtn.textContent = 'Converting...';
    convertBtn.classList.add('loading');

    // Hide previous status
    statusDiv.style.display = 'none';

    // Start progress simulation
    const progressTimer = simulateProgress();

    let convertedGamePath = null;
    let conversionSuccess = false;

    try {
        const response = await fetch('/convert', {
            method: 'POST',
            body: formData
        });

        const result = await response.json();

        clearInterval(progressTimer);
        completeProgress();

        if (result.success) {
            conversionSuccess = true;
            const titleInfo = result.game_title ?
                `<strong>${result.game_title}</strong> (${result.title_id})` :
                `Title ID: ${result.title_id}`;
            statusDiv.innerHTML = `✓ Conversion successful!<br>${titleInfo}<br><pre>${result.message}</pre>`;
            statusDiv.className = 'status-success';
            addToHistory({ name: fileInfo.name, success: true });

            // Use the god_path returned from the server
            convertedGamePath = result.god_path;
        } else {
            statusDiv.innerHTML = `✗ Conversion failed:<br>${result.message}`;
            statusDiv.className = 'status-error';
            addToHistory({ name: fileInfo.name, success: false });
        }

        statusDiv.style.display = 'block';

        // Auto-transfer if enabled and conversion succeeded
        if (autoTransfer && conversionSuccess) {
            // Validate FTP settings
            const ftpHost = document.getElementById('ftp-host-inline').value;
            const ftpPort = parseInt(document.getElementById('ftp-port-inline').value);
            const ftpUsername = document.getElementById('ftp-username-inline').value;
            const ftpPassword = document.getElementById('ftp-password-inline').value;
            const ftpTargetPath = document.getElementById('ftp-target-path-inline').value;
            const passiveMode = document.getElementById('ftp-passive-mode').checked;

            if (!ftpHost) {
                statusDiv.innerHTML += '<br><br>⚠️ Auto-transfer skipped: No Xbox IP address provided';
                statusDiv.className = 'status-info';
                return;
            }

            // Save FTP credentials for next time
            const creds = {
                host: ftpHost,
                port: ftpPort,
                username: ftpUsername,
                password: ftpPassword,
                targetPath: ftpTargetPath,
                passiveMode: passiveMode
            };
            localStorage.setItem('ftpCredentials', JSON.stringify(creds));

            // Update UI for transfer phase
            convertBtn.textContent = 'Transferring to Xbox...';
            resetProgress();

            statusDiv.innerHTML += '<br><br>📡 Starting FTP transfer to Xbox 360...';
            statusDiv.className = 'status-info';

            // Generate a unique session ID for this FTP transfer
            const sessionId = 'ftp-' + Date.now() + '-' + Math.random().toString(36).substr(2, 9);

            // Start listening to FTP progress via SSE
            const progressEventSource = new EventSource(`/ftp-progress/${sessionId}`);
            const progressContainer = document.getElementById('progress-container');
            const progressFill = document.getElementById('progress-fill');
            const progressText = document.getElementById('progress-text');
            const progressMessage = document.getElementById('progress-message');

            progressContainer.style.display = 'block';

            progressEventSource.onmessage = (event) => {
                const progress = JSON.parse(event.data);

                progressFill.style.width = progress.percentage + '%';
                progressText.textContent = progress.percentage + '%';
                progressMessage.textContent = progress.message;

                if (progress.is_complete) {
                    progressEventSource.close();
                }
            };

            progressEventSource.onerror = () => {
                progressEventSource.close();
            };

            try {
                const transferResponse = await fetch('/ftp-transfer', {
                    method: 'POST',
                    headers: {
                        'Content-Type': 'application/json'
                    },
                    body: JSON.stringify({
                        session_id: sessionId,
                        god_path: convertedGamePath,
                        ftp_host: ftpHost,
                        ftp_port: ftpPort,
                        ftp_username: ftpUsername,
                        ftp_password: ftpPassword,
                        ftp_target_path: ftpTargetPath,
                        passive_mode: passiveMode
                    })
                });

                const transferResult = await transferResponse.json();

                progressEventSource.close();
                completeProgress();

                if (transferResult.success) {
                    statusDiv.innerHTML = `✓ Conversion & Transfer Complete!<br><pre>${result.message}</pre><br><br>✓ FTP Transfer: ${transferResult.message}<br>Files transferred: ${transferResult.files_transferred}`;
                    statusDiv.className = 'status-success';
                } else {
                    statusDiv.innerHTML = `✓ Conversion successful<br><pre>${result.message}</pre><br><br>✗ FTP Transfer failed: ${transferResult.message}`;
                    statusDiv.className = 'status-error';
                }
            } catch (transferError) {
                progressEventSource.close();
                resetProgress();
                statusDiv.innerHTML = `✓ Conversion successful<br><pre>${result.message}</pre><br><br>✗ FTP Transfer failed: ${transferError.message}`;
                statusDiv.className = 'status-error';
            }
        }
    } catch (error) {
        clearInterval(progressTimer);
        resetProgress();

        statusDiv.innerHTML = `✗ An error occurred: ${error.message}`;
        statusDiv.className = 'status-error';
        statusDiv.style.display = 'block';

        addToHistory({ name: fileInfo.name, success: false });
    } finally {
        convertBtn.disabled = false;
        convertBtn.textContent = originalButtonText;
        convertBtn.classList.remove('loading');
    }
}

// Batch conversion function
async function convertBatch(filesToConvert, form, statusDiv, convertBtn, autoTransfer) {
    const batchContainer = document.getElementById('batch-progress-container');
    const batchList = document.getElementById('batch-list');
    const batchCurrent = document.getElementById('batch-current');
    const batchTotal = document.getElementById('batch-total');
    
    const originalButtonText = autoTransfer ? 'Convert & Transfer' : 'Convert';
    
    // Disable form
    convertBtn.disabled = true;
    convertBtn.textContent = 'Batch Converting...';
    convertBtn.classList.add('loading');
    
    // Hide single status, show batch progress
    statusDiv.style.display = 'none';
    batchContainer.style.display = 'block';
    batchTotal.textContent = filesToConvert.length;
    batchCurrent.textContent = '0';
    
    // Initialize batch list UI
    batchList.innerHTML = filesToConvert.map((file, index) => `
        <div class="batch-item pending" id="batch-item-${index}">
            <span class="batch-name">📀 ${file.name}</span>
            <span class="batch-status-icon">⏳</span>
        </div>
    `).join('');
    
    let successCount = 0;
    let failCount = 0;
    const results = [];
    
    // Process files sequentially
    for (let i = 0; i < filesToConvert.length; i++) {
        const fileInfo = filesToConvert[i];
        const itemEl = document.getElementById(`batch-item-${i}`);
        
        // Update UI - converting
        itemEl.className = 'batch-item converting';
        itemEl.querySelector('.batch-status-icon').textContent = '🔄';
        batchCurrent.textContent = i + 1;
        
        try {
            const formData = new FormData(form);
            
            if (fileInfo.type === 'path') {
                formData.set('source-iso-path', fileInfo.path);
                formData.delete('source-iso');
            } else {
                formData.set('source-iso', fileInfo.file);
                formData.delete('source-iso-path');
            }
            
            // Clear game title for batch (auto-detect each)
            formData.set('game-title', '');
            
            const response = await fetch('/convert', {
                method: 'POST',
                body: formData
            });
            
            const result = await response.json();
            
            if (result.success) {
                successCount++;
                itemEl.className = 'batch-item success';
                itemEl.querySelector('.batch-status-icon').textContent = '✓';
                addToHistory({ name: fileInfo.name, success: true });
                results.push({ fileInfo, result, success: true });
                
                // Auto-transfer if enabled
                if (autoTransfer && result.god_path) {
                    itemEl.querySelector('.batch-status-icon').textContent = '📤';
                    const transferResult = await doFtpTransfer(result.god_path);
                    if (transferResult.success) {
                        itemEl.querySelector('.batch-status-icon').textContent = '✓';
                    } else {
                        itemEl.querySelector('.batch-status-icon').textContent = '⚠️';
                    }
                }
            } else {
                failCount++;
                itemEl.className = 'batch-item error';
                itemEl.querySelector('.batch-status-icon').textContent = '✗';
                addToHistory({ name: fileInfo.name, success: false });
                results.push({ fileInfo, result, success: false });
            }
        } catch (error) {
            failCount++;
            itemEl.className = 'batch-item error';
            itemEl.querySelector('.batch-status-icon').textContent = '✗';
            addToHistory({ name: fileInfo.name, success: false });
            results.push({ fileInfo, error: error.message, success: false });
        }
    }
    
    // Show final summary
    statusDiv.innerHTML = `
        <strong>📦 Batch Conversion Complete!</strong><br><br>
        ✓ Success: ${successCount} files<br>
        ${failCount > 0 ? `✗ Failed: ${failCount} files` : ''}
    `;
    statusDiv.className = successCount === filesToConvert.length ? 'status-success' : 'status-info';
    statusDiv.style.display = 'block';
    
    // Re-enable form
    convertBtn.disabled = false;
    convertBtn.textContent = originalButtonText;
    convertBtn.classList.remove('loading');
    
    // Clear uploaded files after batch
    uploadedFiles = [];
    renderFileList();
}

// Helper function for FTP transfer
async function doFtpTransfer(godPath) {
    const ftpHost = document.getElementById('ftp-host-inline').value;
    const ftpPort = parseInt(document.getElementById('ftp-port-inline').value);
    const ftpUsername = document.getElementById('ftp-username-inline').value;
    const ftpPassword = document.getElementById('ftp-password-inline').value;
    const ftpTargetPath = document.getElementById('ftp-target-path-inline').value;
    const passiveMode = document.getElementById('ftp-passive-mode').checked;
    
    if (!ftpHost) {
        return { success: false, message: 'No FTP host configured' };
    }
    
    const sessionId = 'ftp-' + Date.now() + '-' + Math.random().toString(36).substr(2, 9);
    
    try {
        const response = await fetch('/ftp-transfer', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({
                session_id: sessionId,
                god_path: godPath,
                ftp_host: ftpHost,
                ftp_port: ftpPort,
                ftp_username: ftpUsername,
                ftp_password: ftpPassword,
                ftp_target_path: ftpTargetPath,
                passive_mode: passiveMode
            })
        });
        return await response.json();
    } catch (error) {
        return { success: false, message: error.message };
    }
}

// Update the Auto option with detected core count
function updateAutoThreadOption() {
    const coreCount = navigator.hardwareConcurrency || 'All';
    const autoOption = document.querySelector('#num-threads option[value="auto"]');
    if (autoOption) {
        autoOption.textContent = `Auto (Use All ${coreCount} Cores)`;
    }
}

// Toggle FTP settings visibility
document.getElementById('auto-transfer').addEventListener('change', function() {
    const ftpSettings = document.getElementById('ftp-settings-inline');
    const convertBtn = document.getElementById('convert-btn');

    if (this.checked) {
        ftpSettings.style.display = 'block';
        convertBtn.textContent = 'Convert & Transfer';
        // Load saved FTP credentials into inline form
        const savedCreds = localStorage.getItem('ftpCredentials');
        if (savedCreds) {
            const creds = JSON.parse(savedCreds);
            document.getElementById('ftp-host-inline').value = creds.host || '';
            document.getElementById('ftp-port-inline').value = creds.port || 21;
            document.getElementById('ftp-username-inline').value = creds.username || 'xbox';
            document.getElementById('ftp-password-inline').value = creds.password || 'xbox';
            document.getElementById('ftp-target-path-inline').value = creds.targetPath || '/Hdd1/Games';
            document.getElementById('ftp-passive-mode').checked = creds.passiveMode || false;
        }
    } else {
        ftpSettings.style.display = 'none';
        convertBtn.textContent = 'Convert';
    }
});

// Test FTP connection button handler
document.getElementById('test-ftp-btn').addEventListener('click', async function() {
    const ftpHost = document.getElementById('ftp-host-inline').value;
    const ftpPort = parseInt(document.getElementById('ftp-port-inline').value);
    const ftpUsername = document.getElementById('ftp-username-inline').value;
    const ftpPassword = document.getElementById('ftp-password-inline').value;
    const passiveMode = document.getElementById('ftp-passive-mode').checked;
    const resultSpan = document.getElementById('ftp-test-result');
    const testBtn = document.getElementById('test-ftp-btn');

    if (!ftpHost) {
        resultSpan.innerHTML = '<span style="color: #c62828;">⚠️ Please enter an IP address</span>';
        return;
    }

    testBtn.disabled = true;
    testBtn.textContent = '🔄 Testing...';
    resultSpan.innerHTML = '<span style="color: #666;">Connecting...</span>';

    try {
        const response = await fetch('/ftp-test', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({
                ftp_host: ftpHost,
                ftp_port: ftpPort,
                ftp_username: ftpUsername,
                ftp_password: ftpPassword,
                passive_mode: passiveMode
            })
        });

        const result = await response.json();

        if (result.success) {
            resultSpan.innerHTML = `<span style="color: #2e7d32;">✓ ${result.message}</span>`;
        } else {
            resultSpan.innerHTML = `<span style="color: #c62828;">✗ ${result.message}</span>`;
        }
    } catch (error) {
        resultSpan.innerHTML = `<span style="color: #c62828;">✗ Error: ${error.message}</span>`;
    } finally {
        testBtn.disabled = false;
        testBtn.textContent = '🔌 Test Connection';
    }
});

// Load ISO files and display history when page loads
loadIsoFiles();
displayHistory();
updateAutoThreadOption();
