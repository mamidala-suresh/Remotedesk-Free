# RemoteDesk Free - GitHub Hosting & Deployment Guide

This guide provides step-by-step instructions for hosting the **RemoteDesk Free** project on your personal or organization GitHub account, setting up automated releases, and publishing downloadable binaries (`.exe` installers and `.pdf` manuals) for your users.

---

## 📋 Table of Contents
1. [Prerequisites](#1-prerequisites)
2. [Step 1: Create a New Repository on GitHub](#step-1-create-a-new-repository-on-github)
3. [Step 2: Upload Your Project to GitHub](#step-2-upload-your-project-to-github)
   - [Option A: Using Git CLI / Terminal (Recommended)](#option-a-using-git-cli--terminal-recommended)
   - [Option B: Using GitHub Desktop (Graphical / No CLI)](#option-b-using-github-desktop-graphical--no-cli)
4. [Step 3: Publish Downloadable Releases (Installers & Executables)](#step-3-publish-downloadable-releases)
5. [Step 4: Automated CI/CD with GitHub Actions](#step-4-automated-cicd-with-github-actions)
6. [Step 5: Recommended Repository Settings & Presentation](#step-5-recommended-repository-settings--presentation)

---

## 1. Prerequisites

Before starting, ensure you have:
1. A **GitHub account** ([Sign up at github.com](https://github.com/join)).
2. One of the following Git tools installed on your computer:
   - **Git for Windows** ([git-scm.com/download/win](https://git-scm.com/download/win)) — *or install via PowerShell:* `winget install --id Git.Git -e`
   - **GitHub Desktop** ([desktop.github.com](https://desktop.github.com)) — *for an easy visual drag-and-drop interface.*

---

## Step 1: Create a New Repository on GitHub

1. Log in to [GitHub](https://github.com).
2. Click the **`+`** icon in the upper-right corner and select **New repository** (or navigate to [github.com/new](https://github.com/new)).
3. Fill in the repository details:
   - **Repository name**: `remotedesk-free` (or your preferred name, e.g. `remote-desk`).
   - **Description**: `Enterprise-grade, lightweight Windows remote desktop and IT assistance application in native Rust.`
   - **Visibility**: Select **Public** (to allow anyone to download releases and view documentation) or **Private** (if intended strictly for internal corporate use).
   - **Initialize this repository with**: **LEAVE ALL UNCHECKED** (Do NOT add a README, .gitignore, or license, as these files are already created in your local project folder).
4. Click **Create repository**.
5. Copy your new repository HTTPS URL (e.g. `https://github.com/YOUR_USERNAME/remotedesk-free.git`).

---

## Step 2: Upload Your Project to GitHub

### Option A: Using Git CLI / Terminal (Recommended)

1. Open **PowerShell** or **Command Prompt** on your computer.
2. Navigate to your project directory:
   ```powershell
   cd "c:\Users\ETP\Downloads\Free Remote"
   ```

3. Initialize Git in the project:
   ```powershell
   git init
   ```

4. Configure your Git name and email (if you haven't already):
   ```powershell
   git config --global user.name "Your Name"
   git config --global user.email "your-email@example.com"
   ```

5. Stage and commit your files:
   ```powershell
   git add .
   git commit -m "Initial release: RemoteDesk Free v1.1.0.1 with WebRTC P2P and PDF Guide"
   ```

6. Set the default branch to `main`:
   ```powershell
   git branch -M main
   ```

7. Connect your local repository to your GitHub repository *(replace `YOUR_USERNAME` with your actual GitHub username)*:
   ```powershell
   git remote add origin https://github.com/YOUR_USERNAME/remotedesk-free.git
   ```

8. Push your code to GitHub:
   ```powershell
   git push -u origin main
   ```
   *(If prompted, log in with your GitHub credentials or a Personal Access Token).*

---

### Option B: Using GitHub Desktop (Graphical / No CLI)

If you prefer a visual interface without typing terminal commands:

1. Download and open **GitHub Desktop** ([desktop.github.com](https://desktop.github.com)).
2. Sign in with your GitHub account.
3. In GitHub Desktop, click **File** → **Add Local Repository...** (or press `Ctrl + O`).
4. Click **Choose...** and browse to:
   `c:\Users\ETP\Downloads\Free Remote`
5. Click **Add Repository**. *(If GitHub Desktop asks "This directory does not appear to be a Git repository. Would you like to create a repository here?", click **Create a repository**)*.
6. In the bottom left, enter a commit message:
   `Initial release: RemoteDesk Free v1.1.0.1`
7. Click the blue **Commit to main** button.
8. Click **Publish repository** in the top bar.
9. Verify the name and click **Publish Repository**.

---

## Step 3: Publish Downloadable Releases

In Git best practice, large binary files (`.exe`, `.pdf`) are distributed to users via **GitHub Releases** rather than cluttering Git commit history.

1. Go to your repository page on GitHub:
   `https://github.com/YOUR_USERNAME/remotedesk-free`
2. On the right-hand sidebar, locate **Releases** and click **Create a new release** (or **Draft a new release**).
3. Fill in the release details:
   - **Tag version**: `v1.1.0.1` (click *"Create new tag"*).
   - **Target**: `main`.
   - **Release title**: `RemoteDesk Free v1.1.0.1 - High-Performance Remote Desktop`
   - **Description**: Paste the following release notes:
     ```markdown
     ### RemoteDesk Free v1.1.0.1 Production Release

     #### 🚀 Highlights & Features:
     - **Sub-15ms Low-Latency Screen Streaming**: Direct WebRTC P2P DataChannel with automatic Worldwide Cloud Mesh fallback.
     - **Unified Dual-Role Architecture**: Single installer handles both Host (to be controlled) and Viewer (controller).
     - **Full System Remote Control**: Hardware mouse mapping and direct keyboard shortcut passthrough (`Win+R`, `Win+E`, `Alt+Tab`, `Ctrl+Shift+Esc`, `Ctrl+Alt+Del`).
     - **Dual-Pane File Manager**: Side-by-side local vs. remote file browser with 64 KB chunking and SHA-256 verification.
     - **Multi-Session Tabs**: Manage multiple client PCs simultaneously with instant tab switching.
     - **Zero Geographical Restrictions**: Works across any ISP, corporate firewall, or country worldwide.
     - **Pure Win32 GUI**: Zero command-line console windows (`#![windows_subsystem = "windows"]`).

     #### 📦 Download Assets:
     - **RemoteDesk-Free-Setup-v1.1.0.1.exe**: Standard Windows Setup Installer (Recommended for all users).
     - **RemoteDesk-Free.exe**: Standalone portable application (runs directly with no installation).
     - **RemoteDesk-Free-User-Guide.pdf**: Complete 8-page Illustrated User Guide & Admin Manual.
     - **RemoteDesk-Server.exe**: Optional self-hosted server for offline private corporate LANs.
     ```
4. Under **Attach binaries by dropping them here or selecting them**, drag and drop these 4 files from your local folder:
   - `RemoteDesk-Free-Setup-v1.1.0.1.exe` *(4.63 MB)*
   - `RemoteDesk-Free.exe` *(2.28 MB)*
   - `RemoteDesk-Free-User-Guide.pdf` *(750 KB)*
   - `RemoteDesk-Server.exe` *(986 KB)*
5. Check **Set as the latest release**.
6. Click **Publish release**.

Your users can now download the installer and guide with one click directly from your GitHub page!

---

## Step 4: Automated CI/CD with GitHub Actions

This repository already includes an automated GitHub Actions workflow file:  
📁 [`.github/workflows/release.yml`](../.github/workflows/release.yml)

### How It Works:
Whenever you create and push a new Git version tag (e.g., `v1.1.0.2`):
```powershell
git tag v1.1.0.2
git push origin v1.1.0.2
```
GitHub's cloud servers will automatically:
1. Check out your code.
2. Compile the Rust binaries (`RemoteDesk-Free.exe` and `RemoteDesk-Server.exe`) with full optimizations.
3. Run the automated test suite (`cargo test`).
4. Generate the 8-page illustrated PDF User Guide (`generate_pdf_guide.js`).
5. Compile the Inno Setup installer (`RemoteDesk-Free-Setup.exe`).
6. Automatically create a GitHub Release and attach all binaries for download!

---

## Step 5: Recommended Repository Settings & Presentation

To make your repository look authoritative and professional:

1. **Add Description and Topics**:
   - On your GitHub repo homepage, click the **⚙️ (Settings cog)** next to the "About" section.
   - **Description**: `Next-generation, high-performance Windows remote desktop and IT assistance application in native Rust.`
   - **Website**: Link to your personal site or release download URL.
   - **Topics**: Add relevant keywords:
     `remote-desktop`, `anydesk-alternative`, `teamviewer-alternative`, `webrtc`, `rust`, `windows`, `sysadmin`, `screen-sharing`, `remote-support`
2. **Enable Discussions & Issues**:
   - Go to **Settings** → **General** → **Features**.
   - Check **Issues** (to receive user bug reports and feature requests).
   - Check **Discussions** (for user Q&A and community support).
3. **Pin the Repository**:
   - Go to your personal GitHub profile (`https://github.com/YOUR_USERNAME`).
   - Click **Customize your pins** and select **remotedesk-free** so it appears prominently on your profile front page.

---

## 🔄 Making Future Updates

Whenever you make improvements or bug fixes in the future, push them to GitHub with:

```powershell
# 1. Check changed files
git status

# 2. Stage changes
git add .

# 3. Commit with a descriptive message
git commit -m "Enhance performance and update documentation"

# 4. Push to GitHub
git push origin main
```
To publish a new official version release, tag it and push:
```powershell
git tag v1.1.0.2
git push origin v1.1.0.2
```
Your GitHub Actions CI/CD will automatically compile and publish the new release!
