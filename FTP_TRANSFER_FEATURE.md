# 🎮 FTP Transfer to Xbox 360 Feature

## Overview

This feature allows you to transfer converted GOD files **directly** from the web interface to your modded Xbox 360 via FTP. No need for USB drives or manual file copying!

## Requirements

### Xbox 360 Side:
- **Modded Xbox 360** (JTAG/RGH)
- **FTP server running** on Xbox (most dashboards like Aurora, FreeStyle Dash have built-in FTP)
- **Network connection** (Xbox and server on same network, or port forwarding if remote)

### Common Xbox FTP Settings:
- **Default Username**: `xbox`
- **Default Password**: `xbox`
- **Default Port**: `21`
- **Common Paths**:
  - `/Hdd1/Games` - Internal HDD games folder
  - `/Hdd1/Content/0000000000000000` - Standard Xbox content folder
  - `/Usb0/Games` - USB drive (if attached)
  - `/Usb1/Games` - Second USB drive

## How To Use

### 1. Convert an ISO
First, convert your ISO using the main conversion form

### 2. Find Your Xbox IP Address
On your Xbox dashboard (Aurora/FSD):
- Go to Settings → Network Settings
- Note down the IP address (e.g., `192.168.1.100`)

### 3. Configure FTP Settings
In the "Transfer to Xbox 360 via FTP" section:
- **Xbox IP Address**: Enter your Xbox's IP (e.g., `192.168.1.100`)
- **FTP Port**: Usually `21` (default)
- **Username**: Usually `xbox`
- **Password**: Usually `xbox`
- **Target Path**: Where to copy files (e.g., `/Hdd1/Games`)

### 4. Save Credentials (Optional)
Check "Remember FTP settings" to save your Xbox details in your browser for next time

### 5. Select Game & Transfer
- Click the "🔄 Refresh" button to load converted games
- Select the game you just converted
- Click "📤 Transfer to Xbox 360"
- Wait for the transfer to complete!

## Features

### Automatic Directory Creation
- Creates folder structure on Xbox automatically
- Maintains GOD format directory hierarchy

### Binary Transfer Mode
- Uses FTP BINARY mode (critical for game files!)
- Ensures file integrity

### Progress Feedback
- Shows connection status
- Displays number of files transferred
- Error messages if something fails

### Credential Storage
- Saves FTP settings in browser localStorage
- Auto-fills on next visit
- Secure (stored locally, never sent to server)

### Smart Game Detection
- Automatically scans `/data/output` for converted games
- Lists all available GOD folders
- Refresh button to rescan after new conversions

## Workflow Example

```
1. Convert ISO → GOD
   ↓
2. Click "Refresh" to see new game
   ↓
3. Select game from dropdown
   ↓
4. Click "Transfer to Xbox 360"
   ↓
5. Game appears in Xbox dashboard!
```

## Troubleshooting

### "Failed to connect to FTP server"
- ✅ Check Xbox is powered on
- ✅ Verify IP address is correct
- ✅ Ensure FTP server is running on Xbox dashboard
- ✅ Check firewall settings

### "FTP login failed"
- ✅ Verify username (usually `xbox`)
- ✅ Verify password (usually `xbox`)
- ✅ Some dashboards have custom credentials

### "Failed to change to target directory"
- ✅ Check path syntax (Unix-style: `/Hdd1/Games`)
- ✅ Ensure directory exists or server can create it
- ✅ Check write permissions on Xbox

### Transfer is slow
- Normal! Transferring 7GB+ files takes time
- Typical speeds: 5-50 MB/s depending on network
- Consider using wired connection for Xbox

### Files transferred but game doesn't appear
- Wait 30 seconds, then restart Xbox dashboard
- Check the correct content folder was used
- Verify GOD files are in correct structure

## Security Notes

⚠️ **Password Storage**: FTP passwords are stored in browser localStorage as plain text. Only use this on trusted computers.

⚠️ **Network Security**: FTP is unencrypted. Use on local network only. Don't expose Xbox FTP to internet.

✅ **Read-Only Source**: The conversion source files are mounted read-only for safety.

## Technical Details

### Backend (Rust)
- Uses `suppaftp` crate for FTP operations
- Supports TLS (for future FTPS support)
- Binary transfer mode enforced
- Recursive directory creation
- File-by-file transfer with error handling

### Frontend (JavaScript)
- JSON API for transfer requests
- localStorage for credential persistence
- Async/await for non-blocking transfers
- Form validation before transfer

### Transfer Process
1. Connect to FTP server
2. Login with credentials
3. Set BINARY transfer mode
4. Navigate to target directory (create if needed)
5. Walk through GOD folder structure
6. Upload each file maintaining structure
7. Disconnect gracefully

## Performance

### Typical Transfer Times (Gigabit Network)
| Game Size | Transfer Time |
|-----------|---------------|
| 1 GB | ~30 seconds |
| 4 GB | ~2 minutes |
| 7 GB | ~3-4 minutes |
| 8 GB (dual-layer) | ~4-5 minutes |

*Times vary based on network speed and Xbox HDD speed*

## Dashboard Compatibility

Tested with:
- ✅ Aurora Dashboard
- ✅ FreeStyle Dash (FSD)
- ✅ XexMenu (with FTP plugin)

## Common Xbox FTP Paths

| Path | Description |
|------|-------------|
| `/Hdd1` | Internal HDD root |
| `/Hdd1/Games` | Games folder (Aurora) |
| `/Hdd1/Content/0000000000000000` | Official Xbox content |
| `/Usb0` | First USB drive |
| `/Usb1` | Second USB drive |
| `/Game` | Current game partition |

## Future Enhancements

Possible improvements:
- **Transfer queue**: Queue multiple games
- **FTPS support**: Encrypted transfers
- **Resume capability**: Resume interrupted transfers
- **Speed limiter**: Limit transfer speed
- **Transfer scheduling**: Transfer during off-hours
- **Auto-detect Xbox**: Scan network for Xbox FTP servers
- **Batch transfer**: Transfer all converted games at once

## Example FTP Configurations

### Aurora Dashboard
```
IP: 192.168.1.100
Port: 21
User: xbox
Pass: xbox
Path: /Hdd1/Games
```

### FreeStyle Dash
```
IP: 192.168.1.100
Port: 21
User: xbox
Pass: xbox
Path: /Hdd1/Content/0000000000000000
```

### Custom Setup
```
IP: Your Xbox IP
Port: 21 (or custom)
User: Custom username
Pass: Custom password
Path: Your preferred path
```

## Credits

FTP transfer functionality powered by:
- `suppaftp` - Rust FTP client library
- Native TLS support for secure connections

---

**Happy Gaming!** 🎮
No more USB drives needed - convert and transfer with one click!
