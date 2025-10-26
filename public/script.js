// Load ISO files on page load
async function loadIsoFiles() {
    try {
        const response = await fetch('/list-isos');
        const isos = await response.json();
        const select = document.getElementById('source-iso-path');

        if (isos.length === 0) {
            select.innerHTML = '<option value="">No ISO files found in /data/input</option>';
        } else {
            select.innerHTML = '<option value="">-- Select an ISO file --</option>';
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
        select.innerHTML = '<option value="">Error loading ISO files</option>';
    }
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
            document.getElementById('source-iso-path').setAttribute('required', 'required');
        } else {
            selectGroup.style.display = 'none';
            uploadGroup.style.display = 'block';
            document.getElementById('source-iso-path').removeAttribute('required');
            document.getElementById('source-iso').setAttribute('required', 'required');
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
    const formData = new FormData(form);
    const statusDiv = document.getElementById('status');
    const convertBtn = document.getElementById('convert-btn');
    const sourceType = document.querySelector('input[name="source-type"]:checked').value;
    const autoTransfer = document.getElementById('auto-transfer').checked;

    // Get filename for history
    let fileName = 'Unknown';
    if (sourceType === 'select') {
        const selectElement = document.getElementById('source-iso-path');
        fileName = selectElement.options[selectElement.selectedIndex].text.split(' (')[0];
        formData.delete('source-iso');
    } else {
        const fileInput = document.getElementById('source-iso');
        if (fileInput.files.length > 0) {
            fileName = fileInput.files[0].name;
        }
        formData.delete('source-iso-path');
    }

    // Update button text based on auto-transfer
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
            statusDiv.innerHTML = `✓ Conversion successful!<br><pre>${result.message}</pre>`;
            statusDiv.className = 'status-success';
            addToHistory({ name: fileName, success: true });

            // Use the god_path returned from the server
            convertedGamePath = result.god_path;
        } else {
            statusDiv.innerHTML = `✗ Conversion failed:<br>${result.message}`;
            statusDiv.className = 'status-error';
            addToHistory({ name: fileName, success: false });
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
                targetPath: ftpTargetPath
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
                        ftp_target_path: ftpTargetPath
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

        addToHistory({ name: fileName, success: false });
    } finally {
        convertBtn.disabled = false;
        convertBtn.textContent = originalButtonText;
        convertBtn.classList.remove('loading');
    }
});

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
        }
    } else {
        ftpSettings.style.display = 'none';
        convertBtn.textContent = 'Convert';
    }
});

// Load ISO files and display history when page loads
loadIsoFiles();
displayHistory();
updateAutoThreadOption();
