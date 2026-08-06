# Himalayas Browser Windows Installation Script
# Supports: Windows 10+ (x86_64 & ARM64)
# Run as Administrator

param(
    [string]$Version = "latest",
    [string]$InstallDir = "C:\Program Files\Himalayas",
    [switch]$AddToPath = $true,
    [switch]$CreateShortcuts = $true,
    [switch]$LaunchAfterInstall = $false
)

# Color output
$ErrorActionPreference = "Stop"
$ProgressPreference = "SilentlyContinue"

function Write-Info {
    param([string]$Message)
    Write-Host "INFO: $Message" -ForegroundColor Green
}

function Write-Warning {
    param([string]$Message)
    Write-Host "WARNING: $Message" -ForegroundColor Yellow
}

function Write-Error {
    param([string]$Message)
    Write-Host "ERROR: $Message" -ForegroundColor Red
    exit 1
}

# Check admin privileges
function Check-Admin {
    $isAdmin = ([Security.Principal.WindowsPrincipal] `
        [Security.Principal.WindowsIdentity]::GetCurrent()). `
        IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)

    if (-not $isAdmin) {
        Write-Error "This script requires Administrator privileges"
    }
}

# Check Windows version
function Check-Windows-Version {
    $osVersion = [Environment]::OSVersion.Version
    if ($osVersion.Major -lt 10) {
        Write-Error "Windows 10 or later is required"
    }
    Write-Info "Windows $($osVersion.Major).$($osVersion.Minor) detected"
}

# Detect architecture
function Get-Architecture {
    $arch = [System.Environment]::Is64BitOperatingSystem ? "x86_64" : "x86"
    if ([Environment]::ProcessorCount -ge 4 -and (Get-CimInstance -ClassName Win32_Processor | Select-Object -First 1).Architecture -eq 12) {
        $arch = "aarch64"
    }
    Write-Info "Architecture: $arch"
    return $arch
}

# Download binary/installer
function Download-Installer {
    param(
        [string]$Version,
        [string]$Architecture
    )

    Write-Info "Downloading Himalayas Browser v${Version}..."

    $downloadDir = "$env:TEMP\himalayas-install"
    New-Item -ItemType Directory -Path $downloadDir -Force | Out-Null

    $msiUrl = "https://github.com/Mullassery/Himalayas/releases/download/v${Version}/himalayas-${Version}-${Architecture}.msi"
    $msiPath = "$downloadDir\himalayas-$Version.msi"

    try {
        Invoke-WebRequest -Uri $msiUrl -OutFile $msiPath -UseBasicParsing
        Write-Info "Downloaded successfully: $msiPath"
        return $msiPath
    }
    catch {
        Write-Error "Failed to download installer: $_"
    }
}

# Install MSI
function Install-MSI {
    param([string]$MsiPath)

    Write-Info "Installing from MSI: $MsiPath"

    $logPath = "$env:TEMP\himalayas-install.log"
    $arguments = @(
        "/i", "$MsiPath",
        "/quiet",
        "/norestart",
        "/l*v", "$logPath",
        "INSTALLFOLDER=""$InstallDir\"""
    )

    try {
        Start-Process -FilePath "msiexec.exe" -ArgumentList $arguments -Wait -NoNewWindow
        Write-Info "Installation completed"
        Write-Info "Log file: $logPath"
    }
    catch {
        Write-Error "MSI installation failed: $_"
    }
}

# Create shortcuts
function Create-Shortcuts {
    Write-Info "Creating shortcuts..."

    $desktopPath = [Environment]::GetFolderPath("Desktop")
    $startMenuPath = [Environment]::GetFolderPath("StartMenu")
    $exePath = "$InstallDir\himalayas.exe"

    # Desktop shortcut
    if (Test-Path $exePath) {
        $shellLink = New-Object -ComObject WScript.Shell
        $shortcut = $shellLink.CreateShortcut("$desktopPath\Himalayas Browser.lnk")
        $shortcut.TargetPath = $exePath
        $shortcut.Description = "The world's first truly agent-native browser platform"
        $shortcut.IconLocation = "$exePath,0"
        $shortcut.Save()
        Write-Info "Desktop shortcut created"

        # Start Menu shortcut
        $startMenuDir = "$startMenuPath\Programs\Himalayas"
        New-Item -ItemType Directory -Path $startMenuDir -Force | Out-Null
        $shortcut = $shellLink.CreateShortcut("$startMenuDir\Himalayas Browser.lnk")
        $shortcut.TargetPath = $exePath
        $shortcut.Description = "The world's first truly agent-native browser platform"
        $shortcut.IconLocation = "$exePath,0"
        $shortcut.Save()
        Write-Info "Start Menu shortcut created"
    }
}

# Add to PATH
function Add-To-Path {
    Write-Info "Adding to PATH..."

    $binDir = "$InstallDir"
    $currentPath = [Environment]::GetEnvironmentVariable("Path", "Machine")

    if ($currentPath -notlike "*$binDir*") {
        $newPath = "$currentPath;$binDir"
        [Environment]::SetEnvironmentVariable("Path", $newPath, "Machine")
        Write-Info "Added to PATH: $binDir"
    }
}

# Register file associations
function Register-FileAssociations {
    Write-Info "Registering file associations..."

    $exePath = "$InstallDir\himalayas.exe"

    if (Test-Path $exePath) {
        $regPath = "HKCU:\Software\Microsoft\Windows\CurrentVersion\Explorer\FileExts"

        # Register for HTML
        reg add "HKCU\Software\Microsoft\Windows\CurrentVersion\ApplicationAssociationToasts" `
            /v "Himalayas.Browser.1_x64.html.html" /t REG_DWORD /d 0 /f 2>$null

        Write-Info "File associations registered"
    }
}

# Set as default browser (optional)
function Set-Default-Browser {
    Write-Info "Configuring as default browser option..."

    $exePath = "$InstallDir\himalayas.exe"

    if (Test-Path $exePath) {
        # Register protocol handlers
        reg add "HKCU\Software\Microsoft\Windows\CurrentVersion\Explorer\FileExts\.html\UserChoice" `
            /d "Himalayas.Browser" /f 2>$null
    }
}

# Create uninstaller
function Create-Uninstaller {
    Write-Info "Creating uninstaller..."

    $uninstallerDir = "$InstallDir"
    $uninstallerScript = @"
`$InstallDir = "$InstallDir"
if (Test-Path `$InstallDir) {
    Remove-Item -Recurse -Force `$InstallDir
}
`$desktopPath = [Environment]::GetFolderPath("Desktop")
Remove-Item -Force "`$desktopPath\Himalayas Browser.lnk" -ErrorAction SilentlyContinue
`$startMenuPath = [Environment]::GetFolderPath("StartMenu")
Remove-Item -Recurse -Force "`$startMenuPath\Programs\Himalayas" -ErrorAction SilentlyContinue
"@

    $uninstallerScript | Out-File -FilePath "$uninstallerDir\Uninstall.ps1" -Encoding UTF8
    Write-Info "Uninstaller created: $uninstallerDir\Uninstall.ps1"
}

# Verify installation
function Verify-Installation {
    Write-Info "Verifying installation..."

    $exePath = "$InstallDir\himalayas.exe"

    if (-not (Test-Path $exePath)) {
        Write-Error "Installation verification failed: executable not found"
    }

    Write-Info "Installation verified"
}

# Launch application
function Launch-Application {
    Write-Info "Launching Himalayas Browser..."
    Start-Process -FilePath "$InstallDir\himalayas.exe"
}

# Main installation flow
function Main {
    Write-Host ""
    Write-Host "=== Himalayas Browser Installation (Windows) ===" -ForegroundColor Green
    Write-Host ""

    Check-Admin
    Check-Windows-Version
    $arch = Get-Architecture

    $msiPath = Download-Installer -Version $Version -Architecture $arch
    Install-MSI -MsiPath $msiPath

    if ($CreateShortcuts) {
        Create-Shortcuts
    }

    if ($AddToPath) {
        Add-To-Path
    }

    Register-FileAssociations
    Set-Default-Browser
    Create-Uninstaller
    Verify-Installation

    Write-Host ""
    Write-Host "=== Installation Complete ===" -ForegroundColor Green
    Write-Host "Start Himalayas Browser with:"
    Write-Host "  • himalayas (command line)"
    Write-Host "  • Desktop shortcut"
    Write-Host "  • Start Menu → Programs → Himalayas"
    Write-Host "Documentation: https://github.com/Mullassery/Himalayas"
    Write-Host ""

    if ($LaunchAfterInstall) {
        Launch-Application
    }
}

Main
