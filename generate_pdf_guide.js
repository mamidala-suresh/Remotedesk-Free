/**
 * RemoteDesk Free - Enterprise PDF User Guide & Administrator Manual Generator
 * Generates an executive-grade, multi-page vector PDF manual compliant with PDF 1.4.
 * Includes embedded high-resolution screenshots, UI diagrams, and complete functionality guide.
 * Zero external dependencies.
 */

const fs = require('fs');
const path = require('path');

function getJpegDimensions(buffer) {
    let offset = 2;
    while (offset < buffer.length) {
        if (buffer[offset] !== 0xFF) break;
        const marker = buffer[offset + 1];
        if (marker === 0xC0 || marker === 0xC2) {
            const height = buffer.readUInt16BE(offset + 5);
            const width = buffer.readUInt16BE(offset + 7);
            return { width, height };
        }
        const length = buffer.readUInt16BE(offset + 2);
        offset += 2 + length;
    }
    return null;
}

class PDFBuilder {
    constructor() {
        this.pages = [];
        this.currentPageContent = [];
        this.pageWidth = 612;  // US Letter 8.5"
        this.pageHeight = 792; // US Letter 11"
        this.margin = 54;      // 0.75 in margins
        this.contentWidth = this.pageWidth - (this.margin * 2);
        this.cursorY = this.pageHeight - this.margin;
        this.pageNumber = 1;
        this.totalPages = 1;
        this.images = {};
    }

    registerImage(name, filePath) {
        try {
            if (fs.existsSync(filePath)) {
                const buf = fs.readFileSync(filePath);
                const dims = getJpegDimensions(buf);
                if (dims) {
                    this.images[name] = {
                        name: name,
                        width: dims.width,
                        height: dims.height,
                        buffer: buf
                    };
                    console.log(`[PDF] Registered image '${name}': ${dims.width}x${dims.height} (${buf.length} bytes)`);
                }
            }
        } catch (e) {
            console.warn(`[PDF] Failed to register image '${name}':`, e);
        }
    }

    startNewPage() {
        if (this.currentPageContent.length > 0) {
            this.pages.push(this.currentPageContent.join('\n'));
            this.currentPageContent = [];
            this.pageNumber++;
        }
        this.cursorY = this.pageHeight - this.margin;
    }

    checkPageBreak(requiredHeight) {
        if (this.cursorY - requiredHeight < this.margin + 40) { // leave room for footer
            this.startNewPage();
            this.drawHeader();
        }
    }

    esc(text) {
        if (!text) return '';
        return String(text)
            .replace(/\\/g, '\\\\')
            .replace(/\(/g, '\\(')
            .replace(/\)/g, '\\)');
    }

    drawHeader() {
        const y = this.pageHeight - 36;
        this.currentPageContent.push(`0.78 0.83 0.88 RG 0.75 w 54 ${y - 4} m 558 ${y - 4} l S`);
        this.currentPageContent.push(`BT /F2 8 Tf 0.38 0.45 0.54 rg 54 ${y} Td (RemoteDesk Free v1.1.0.1  |  Enterprise User Guide & Admin Manual) Tj ET`);
        this.currentPageContent.push(`BT /F1 8 Tf 0.38 0.45 0.54 rg 430 ${y} Td (Freeware | Suresh Mamidala) Tj ET`);
        this.cursorY = this.pageHeight - 56;
    }

    drawCoverPage() {
        const bannerH = 260;
        const bannerY = this.pageHeight - bannerH;
        this.currentPageContent.push(`0.10 0.22 0.55 rg 0 ${bannerY} ${this.pageWidth} ${bannerH} re f`);
        this.currentPageContent.push(`0.02 0.52 0.78 rg 0 ${this.pageHeight - 6} ${this.pageWidth} 6 re f`);
        this.currentPageContent.push(`0.14 0.30 0.68 rg 0 ${bannerY} 612 10 re f`);

        // App Logo Icon
        const logoX = 54;
        const logoY = this.pageHeight - 110;
        if (this.images['AppIcon']) {
            this.currentPageContent.push(`1 1 1 rg ${logoX - 2} ${logoY - 2} 52 52 re f`);
            this.currentPageContent.push(`q 48 0 0 48 ${logoX} ${logoY} cm /AppIcon Do Q`);
        } else {
            this.currentPageContent.push(`1 1 1 rg ${logoX} ${logoY} 48 48 re f`);
            this.currentPageContent.push(`0.02 0.52 0.78 rg ${logoX + 4} ${logoY + 4} 40 40 re f`);
        }

        // App Title
        this.currentPageContent.push(`BT /F2 26 Tf 1 1 1 rg 116 ${this.pageHeight - 85} Td (RemoteDesk Free) Tj ET`);
        this.currentPageContent.push(`BT /F2 10 Tf 0.40 0.80 1.00 rg 365 ${this.pageHeight - 82} Td ([ v1.1.0.1 Enterprise ]) Tj ET`);
        this.currentPageContent.push(`BT /F1 12.5 Tf 0.85 0.92 1.00 rg 116 ${this.pageHeight - 104} Td (Next-Gen Windows 10/11 Remote Desktop & IT Support Solution) Tj ET`);

        this.currentPageContent.push(`BT /F1 10.5 Tf 0.90 0.95 1.00 rg 54 ${this.pageHeight - 165} Td (Comprehensive Operational Manual, System Installation & Security Guide) Tj ET`);
        this.currentPageContent.push(`BT /F1 9 Tf 0.75 0.85 0.98 rg 54 ${this.pageHeight - 182} Td (Dual-Role Unified App * Zero Console Window * WebRTC Direct P2P * Dual-Pane File Manager) Tj ET`);

        // Author Pill
        this.currentPageContent.push(`0.06 0.15 0.40 rg 54 ${this.pageHeight - 232} 504 32 re f`);
        this.currentPageContent.push(`BT /F2 9.5 Tf 1 1 1 rg 66 ${this.pageHeight - 220} Td (Company: Freeware | Suresh Mamidala) Tj ET`);
        this.currentPageContent.push(`BT /F2 9.5 Tf 0.50 0.85 1.00 rg 330 ${this.pageHeight - 220} Td (Publisher / Author: Suresh Mamidala) Tj ET`);

        this.cursorY = this.pageHeight - 290;

        this.addSectionHeader("Executive Summary & Capabilities", true);
        this.addParagraph("RemoteDesk Free (v1.1.0.1) is an enterprise-class, lightweight Windows desktop application engineered in high-performance native Rust. It provides secure, low-latency, end-to-end encrypted remote desktop access, interactive mouse and keyboard remote control, dual-pane cryptographic file transfer, multi-session tab management, and persistent address book organization across Windows 10 and Windows 11.");

        // 4 Feature Pillars Grid
        this.checkPageBreak(170);
        const colW = (this.contentWidth - 16) / 2;
        const boxH = 68;

        this.drawCard(this.margin, this.cursorY - boxH, colW, boxH, "High-Speed Screen Streaming", "Dynamic delta-tiling with zero-allocation RGB encoding delivers fluid 30-60 FPS remote display with sub-20ms local network latency.", [0.93, 0.96, 1.0], [0.15, 0.40, 0.85]);
        this.drawCard(this.margin + colW + 16, this.cursorY - boxH, colW, boxH, "Full System Remote Control", "Exact mouse injection and layout-independent Unicode text entry with quick system actions (Ctrl+Alt+Del, Task Manager, Win+D, Win+L).", [0.93, 0.98, 0.95], [0.08, 0.60, 0.35]);

        this.cursorY -= (boxH + 12);

        this.drawCard(this.margin, this.cursorY - boxH, colW, boxH, "Dual-Pane File Transfer", "Cryptographically verified (SHA-256) 64 KB chunked file transfer engine with direct file upload and progress tracking.", [0.98, 0.95, 1.0], [0.55, 0.25, 0.85]);
        this.drawCard(this.margin + colW + 16, this.cursorY - boxH, colW, boxH, "Address Book & Unattended", "Persistent address book with group tagging, instant search, and PBKDF2/SHA-256 salted password security for 24/7 IT management.", [1.0, 0.97, 0.92], [0.85, 0.45, 0.08]);

        this.cursorY -= (boxH + 20);

        this.addCallout("DEPLOYMENT READY", "Packaged as a modern Windows 10/11 Inno Setup installer (RemoteDesk-Free-Setup-v1.1.0.1.exe) and standalone zero-dependency portable executable (RemoteDesk-Free.exe). Compliant with Windows PE specifications, including embedded Application Manifest (asInvoker) and PE VersionInfo (1.1.0.1).", "info");

        this.startNewPage();
        this.drawHeader();
    }

    drawCard(x, y, w, h, title, desc, bgRgb, borderRgb) {
        this.currentPageContent.push(`${bgRgb[0]} ${bgRgb[1]} ${bgRgb[2]} rg ${x} ${y} ${w} ${h} re f`);
        this.currentPageContent.push(`${borderRgb[0]} ${borderRgb[1]} ${borderRgb[2]} rg ${x} ${y} 4 ${h} re f`);
        this.currentPageContent.push(`0.85 0.88 0.92 RG 0.5 w ${x} ${y} ${w} ${h} re S`);
        this.currentPageContent.push(`BT /F2 10 Tf ${borderRgb[0]} ${borderRgb[1]} ${borderRgb[2]} rg ${x + 10} ${y + h - 16} Td (${this.esc(title)}) Tj ET`);
        const lines = this.wrapText(desc, w - 20, 8);
        let lineY = y + h - 30;
        for (const line of lines.slice(0, 3)) {
            this.currentPageContent.push(`BT /F1 8 Tf 0.22 0.28 0.38 rg ${x + 10} ${lineY} Td (${this.esc(line)}) Tj ET`);
            lineY -= 11;
        }
    }

    addSectionHeader(title, isMajor = false) {
        const h = isMajor ? 32 : 24;
        this.checkPageBreak(h + 20);
        this.cursorY -= 8;

        if (isMajor) {
            this.currentPageContent.push(`0.15 0.38 0.92 rg 54 ${this.cursorY - 2} 4 18 re f`);
            this.currentPageContent.push(`BT /F2 15 Tf 0.06 0.12 0.25 rg 64 ${this.cursorY} Td (${this.esc(title)}) Tj ET`);
            this.cursorY -= 20;
        } else {
            this.currentPageContent.push(`BT /F2 11.5 Tf 0.12 0.24 0.55 rg 54 ${this.cursorY} Td (${this.esc(title)}) Tj ET`);
            this.cursorY -= 16;
        }
    }

    addSubHeader(title) {
        this.checkPageBreak(24);
        this.cursorY -= 6;
        this.currentPageContent.push(`BT /F2 10 Tf 0.20 0.30 0.45 rg 54 ${this.cursorY} Td (${this.esc(title)}) Tj ET`);
        this.cursorY -= 14;
    }

    addParagraph(text) {
        const lines = this.wrapText(text, this.contentWidth, 9.5);
        this.checkPageBreak(lines.length * 13 + 6);
        for (const line of lines) {
            this.currentPageContent.push(`BT /F1 9.5 Tf 0.15 0.20 0.28 rg 54 ${this.cursorY} Td (${this.esc(line)}) Tj ET`);
            this.cursorY -= 13;
        }
        this.cursorY -= 5;
    }

    addBullet(bulletText, boldPrefix = '') {
        const fullText = boldPrefix ? `${boldPrefix}: ${bulletText}` : bulletText;
        const lines = this.wrapText(fullText, this.contentWidth - 18, 9);
        this.checkPageBreak(lines.length * 12.5 + 4);

        this.currentPageContent.push(`0.15 0.38 0.92 rg 60 ${this.cursorY + 3} 3.5 3.5 re f`);

        let first = true;
        for (const line of lines) {
            const font = (first && boldPrefix) ? '/F2' : '/F1';
            this.currentPageContent.push(`BT ${font} 9 Tf 0.18 0.24 0.32 rg 70 ${this.cursorY} Td (${this.esc(line)}) Tj ET`);
            this.cursorY -= 12.5;
            first = false;
        }
        this.cursorY -= 2;
    }

    addCallout(title, body, type = 'info') {
        const lines = this.wrapText(body, this.contentWidth - 28, 8.5);
        const boxH = lines.length * 12 + 32;
        this.checkPageBreak(boxH + 10);

        let bg = [0.93, 0.96, 1.0];
        let border = [0.15, 0.40, 0.85];
        let titleColor = [0.10, 0.25, 0.65];

        if (type === 'success') {
            bg = [0.92, 0.98, 0.94];
            border = [0.08, 0.65, 0.35];
            titleColor = [0.05, 0.45, 0.22];
        } else if (type === 'warning') {
            bg = [1.0, 0.97, 0.92];
            border = [0.90, 0.55, 0.10];
            titleColor = [0.70, 0.35, 0.05];
        }

        const boxY = this.cursorY - boxH;
        this.currentPageContent.push(`${bg[0]} ${bg[1]} ${bg[2]} rg 54 ${boxY} ${this.contentWidth} ${boxH} re f`);
        this.currentPageContent.push(`${border[0]} ${border[1]} ${border[2]} rg 54 ${boxY} 4 ${boxH} re f`);
        this.currentPageContent.push(`0.85 0.88 0.92 RG 0.5 w 54 ${boxY} ${this.contentWidth} ${boxH} re S`);
        this.currentPageContent.push(`BT /F2 9.5 Tf ${titleColor[0]} ${titleColor[1]} ${titleColor[2]} rg 68 ${this.cursorY - 16} Td (${this.esc(title)}) Tj ET`);

        let curY = this.cursorY - 29;
        for (const line of lines) {
            this.currentPageContent.push(`BT /F1 8.5 Tf 0.18 0.24 0.32 rg 68 ${curY} Td (${this.esc(line)}) Tj ET`);
            curY -= 12;
        }

        this.cursorY = boxY - 10;
    }

    addTable(headers, rows, colWidths) {
        const rowH = 18;
        const totalH = (rows.length + 1) * rowH;
        this.checkPageBreak(totalH + 15);

        let curY = this.cursorY;
        const startX = 54;

        this.currentPageContent.push(`0.08 0.15 0.30 rg ${startX} ${curY - rowH} ${this.contentWidth} ${rowH} re f`);

        let curX = startX;
        for (let c = 0; c < headers.length; c++) {
            const w = colWidths[c];
            this.currentPageContent.push(`BT /F2 8.5 Tf 1 1 1 rg ${curX + 6} ${curY - 13} Td (${this.esc(headers[c])}) Tj ET`);
            curX += w;
        }
        curY -= rowH;

        for (let r = 0; r < rows.length; r++) {
            const row = rows[r];
            const bgRgb = (r % 2 === 0) ? [0.97, 0.98, 1.0] : [1, 1, 1];
            this.currentPageContent.push(`${bgRgb[0]} ${bgRgb[1]} ${bgRgb[2]} rg ${startX} ${curY - rowH} ${this.contentWidth} ${rowH} re f`);
            this.currentPageContent.push(`0.88 0.90 0.94 RG 0.5 w ${startX} ${curY - rowH} ${this.contentWidth} ${rowH} re S`);

            curX = startX;
            for (let c = 0; c < row.length; c++) {
                const w = colWidths[c];
                const font = (c === 0) ? '/F2' : '/F1';
                this.currentPageContent.push(`BT ${font} 8 Tf 0.15 0.20 0.28 rg ${curX + 6} ${curY - 13} Td (${this.esc(row[c])}) Tj ET`);
                curX += w;
            }
            curY -= rowH;
        }

        this.cursorY = curY - 12;
    }

    addImage(imgKey, maxDisplayH = 220, caption = '') {
        const img = this.images[imgKey];
        if (!img) return;

        // Calculate aspect ratio fit inside contentWidth and maxDisplayH
        const aspect = img.width / img.height;
        let displayW = this.contentWidth;
        let displayH = displayW / aspect;

        if (displayH > maxDisplayH) {
            displayH = maxDisplayH;
            displayW = displayH * aspect;
        }

        const totalBlockH = displayH + (caption ? 24 : 10) + 12;
        this.checkPageBreak(totalBlockH);

        const imgX = this.margin + ((this.contentWidth - displayW) / 2);
        const imgY = this.cursorY - displayH - 6;

        // Frame and subtle shadow outline
        this.currentPageContent.push(`0.90 0.92 0.96 rg ${imgX - 3} ${imgY - 3} ${displayW + 6} ${displayH + 6} re f`);
        this.currentPageContent.push(`0.75 0.80 0.88 RG 0.75 w ${imgX - 3} ${imgY - 3} ${displayW + 6} ${displayH + 6} re S`);

        // Draw Image XObject
        this.currentPageContent.push(`q ${displayW.toFixed(2)} 0 0 ${displayH.toFixed(2)} ${imgX.toFixed(2)} ${imgY.toFixed(2)} cm /${imgKey} Do Q`);

        // Caption
        if (caption) {
            const capY = imgY - 14;
            this.currentPageContent.push(`BT /F2 8 Tf 0.30 0.38 0.50 rg ${this.margin} ${capY} Td (${this.esc('Screenshot: ' + caption)}) Tj ET`);
            this.cursorY = capY - 12;
        } else {
            this.cursorY = imgY - 12;
        }
    }

    wrapText(text, maxWidth, fontSize) {
        if (!text) return [];
        const words = text.split(/\s+/);
        const lines = [];
        let curLine = '';
        const charWidth = fontSize * 0.52;

        for (const word of words) {
            const testLine = curLine ? `${curLine} ${word}` : word;
            if (testLine.length * charWidth > maxWidth) {
                if (curLine) lines.push(curLine);
                curLine = word;
            } else {
                curLine = testLine;
            }
        }
        if (curLine) lines.push(curLine);
        return lines;
    }

    compile() {
        if (this.currentPageContent.length > 0) {
            this.pages.push(this.currentPageContent.join('\n'));
        }
        this.totalPages = this.pages.length;

        for (let i = 1; i < this.pages.length; i++) {
            const pageNum = i + 1;
            const y = 36;
            const footerStream = [
                `0.78 0.83 0.88 RG 0.75 w 54 ${y + 12} m 558 ${y + 12} l S`,
                `BT /F1 8 Tf 0.45 0.50 0.60 rg 54 ${y} Td (RemoteDesk Free v1.1.0.1  *  Freeware | Suresh Mamidala  *  Target: Windows 10 & 11) Tj ET`,
                `BT /F2 8 Tf 0.15 0.38 0.92 rg 505 ${y} Td (Page ${pageNum} of ${this.totalPages}) Tj ET`
            ].join('\n');
            this.pages[i] = `${this.pages[i]}\n${footerStream}`;
        }

        const offsets = [];
        let parts = [];

        function addObj(id, header, streamBuf) {
            const start = parts.reduce((acc, p) => acc + p.length, 0);
            offsets[id] = start;
            let head = Buffer.from(`${id} 0 obj\n${header}\n`, 'latin1');
            parts.push(head);
            if (streamBuf) {
                parts.push(Buffer.from('stream\n', 'latin1'));
                parts.push(streamBuf);
                parts.push(Buffer.from('\nendstream\n', 'latin1'));
            }
            parts.push(Buffer.from('endobj\n', 'latin1'));
        }

        parts.push(Buffer.from('%PDF-1.4\n%\xE2\xE3\xCF\xD3\n', 'latin1'));

        const catalogId = 1;
        const pagesId = 2;
        const fontRegId = 3;
        const fontBoldId = 4;

        let nextId = 5;

        // Image objects
        const imageIds = {};
        for (const [key, img] of Object.entries(this.images)) {
            const id = nextId++;
            imageIds[key] = id;
            const header = `<< /Type /XObject /Subtype /Image /Width ${img.width} /Height ${img.height} /ColorSpace /DeviceRGB /BitsPerComponent 8 /Filter /DCTDecode /Length ${img.buffer.length} >>`;
            addObj(id, header, img.buffer);
        }

        // XObject resource dictionary string
        const xobjEntries = Object.entries(imageIds).map(([k, id]) => `/${k} ${id} 0 R`).join(' ');
        const xobjDict = xobjEntries ? `/XObject << ${xobjEntries} >>` : '';

        const pageObjIds = [];
        const contentObjIds = [];

        for (let i = 0; i < this.pages.length; i++) {
            const pId = nextId++;
            const cId = nextId++;
            pageObjIds.push(pId);
            contentObjIds.push(cId);

            const contentStream = this.pages[i];
            const contentBuf = Buffer.from(contentStream, 'latin1');

            addObj(pId, `<< /Type /Page /Parent ${pagesId} 0 R /MediaBox [0 0 612 792] /Contents ${cId} 0 R /Resources << /Font << /F1 ${fontRegId} 0 R /F2 ${fontBoldId} 0 R >> ${xobjDict} /ProcSet [/PDF /Text /ImageB /ImageC] >> >>`);
            addObj(cId, `<< /Length ${contentBuf.length} >>`, contentBuf);
        }

        addObj(catalogId, `<< /Type /Catalog /Pages ${pagesId} 0 R >>`);
        addObj(pagesId, `<< /Type /Pages /Kids [${pageObjIds.map(id => `${id} 0 R`).join(' ')}] /Count ${pageObjIds.length} >>`);
        addObj(fontRegId, `<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>`);
        addObj(fontBoldId, `<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica-Bold /Encoding /WinAnsiEncoding >>`);

        const maxId = nextId - 1;
        const startXref = parts.reduce((acc, p) => acc + p.length, 0);

        let xrefStr = `xref\n0 ${maxId + 1}\n0000000000 65535 f \n`;
        for (let id = 1; id <= maxId; id++) {
            xrefStr += `${String(offsets[id] || 0).padStart(10, '0')} 00000 n \n`;
        }

        xrefStr += `trailer\n<< /Size ${maxId + 1} /Root ${catalogId} 0 R /Info << /Title (RemoteDesk Free User Guide & Administrator Manual) /Author (Suresh Mamidala) /Creator (RemoteDesk Engine) >> >>\nstartxref\n${startXref}\n%%EOF\n`;
        parts.push(Buffer.from(xrefStr, 'latin1'));

        return Buffer.concat(parts);
    }
}

// -------------------------------------------------------------
// Document Content Authoring
// -------------------------------------------------------------
const doc = new PDFBuilder();

// Register Screenshots from docs_images
const imgDir = path.join(__dirname, 'docs_images');
doc.registerImage('AppIcon', path.join(imgDir, 'app_icon.jpg'));
doc.registerImage('DashboardUI', path.join(imgDir, 'media_1792786346518_4b00dfdb.jpg'));
doc.registerImage('SecurityUI', path.join(imgDir, 'media_1792787370202_74911d0c.jpg'));
doc.registerImage('RemoteSession', path.join(imgDir, 'media_1792789068569_c2f64008.jpg'));
doc.registerImage('ToolbarBadges', path.join(imgDir, 'media_1792824495804_92431d24.jpg'));

// 1. Cover Page
doc.drawCoverPage();

// 2. Section 1: Overview & Architecture
doc.addSectionHeader("1. Architectural Overview & System Design", true);
doc.addParagraph("RemoteDesk Free is designed from the ground up for zero-latency IT desktop management and remote assistance across Windows 10 and Windows 11. It eliminates heavyweight runtimes by pairing a low-level native Rust system engine with a modern, embedded WebView2 interface.");

doc.addSubHeader("Core Engine Specifications");
doc.addBullet("Compiles directly to machine code with LTO (Link-Time Optimization) and zero runtime console windows (#![windows_subsystem = 'windows']).", "Native Zero-Dependency Build");
doc.addBullet("Sub-pixel accurate GDI BitBlt and hardware cursor capture with real-time 1024-point sampling delta detection.", "Optimized Capture Pipeline");
doc.addBullet("Direct WebRTC P2P DataChannel connection with sub-15ms latency and automatic MQTT Cloud Relay fallback.", "Dual-Mode High-Speed Transport");
doc.addBullet("Worldwide multi-broker mesh (EMQX, HiveMQ, Eclipse Foundation port 443, Mosquitto) and global STUN/TURN relays ensure zero geographical restrictions across any country or ISP worldwide.", "Global No-Geo-Restrictions Mesh");
doc.addBullet("Native Per-Monitor DPI Awareness V2 context ensures 100% pixel-for-pixel mouse pointer mapping across 100%, 125%, 150%, and 200% Windows scaling.", "DPI Scaling V2");

doc.addCallout("Zero Cloud Telemetry & Complete Privacy", "Unlike commercial remote desktop solutions that enforce restrictive licensing or upload background analytics, RemoteDesk Free operates with complete privacy. Session pins and authentication keys are hashed with salted PBKDF2 and never transmitted unencrypted.", "success");

// 3. Section 2: Installation & Deployment (Which File to Install Where)
doc.addSectionHeader("2. Installation & Deployment Guide: Which File to Install Where", true);
doc.addParagraph("A common question from users and system administrators is: 'Which file do I install on my computer versus the client computer?'");

doc.addCallout("UNIFIED DUAL-ROLE APPLICATION", "Just like AnyDesk or TeamViewer, RemoteDesk Free is a unified dual-role application. You install or run the exact same program on BOTH your system and the client system! The single executable contains both the Host Agent (system to be controlled) and the Viewer Client (controlling system).", "info");

doc.addSubHeader("The Two Simple Deployment Options");
doc.addBullet("Run 'RemoteDesk-Free-Setup-v1.1.0.1.exe' on both computers. Installs cleanly to C:\\Program Files\\RemoteDesk Free, adds Desktop and Start Menu shortcuts, and integrates standard Windows uninstallation.", "Option A: Standard Windows Installer (Recommended)");
doc.addBullet("Simply copy 'RemoteDesk-Free.exe' to any folder, USB flash drive, or network share on both computers and double-click to run. No installation or admin rights required.", "Option B: Portable Standalone Binary");

doc.addSubHeader("Package Distribution Matrix");
doc.addTable(
    ["Package Binary", "Size", "Installed On", "Operational Role"],
    [
        ["RemoteDesk-Free-Setup-v1.1.0.1.exe", "3.98 MB", "Both Your PC & Client PC", "Complete Freeware Installer with shortcuts & uninstaller"],
        ["RemoteDesk-Free.exe", "2.28 MB", "Both Your PC & Client PC", "Portable standalone GUI app (Host + Viewer in one)"],
        ["RemoteDesk-Server.exe", "986 KB", "Optional (Private LAN only)", "Optional on-prem signaling service for offline airgapped networks"]
    ],
    [175, 55, 140, 142]
);

doc.addCallout("Do You Need RemoteDesk-Server.exe?", "NO! You DO NOT need to install or run RemoteDesk-Server.exe for normal internet or inter-network connections. RemoteDesk Free includes automatic global cloud mesh rendezvous (EMQX / HiveMQ / WebRTC STUN/TURN). RemoteDesk-Server.exe is strictly an optional utility for corporate airgapped intranets with zero internet access.", "warning");

// 4. Section 3: Connecting & Main Dashboard Walkthrough (With Screenshot)
doc.addSectionHeader("3. Connecting & Main Dashboard Walkthrough", true);
doc.addParagraph("RemoteDesk Free simplifies remote desktop connection using a clean, modern dashboard divided into two primary operational zones: 'Your Desk' (Host) and 'Control Remote Desk' (Viewer).");

doc.addImage('DashboardUI', 210, 'Main Dashboard UI: Host Desk ID & PIN (Left) and Viewer Control Remote Desk (Right)');

doc.addSubHeader("Step-by-Step Connection Instructions");
doc.addBullet("Launch RemoteDesk Free on the client computer. The left card ('Your Desk') displays their unique 9-digit Device ID (e.g. 660 856 593) and a dynamic 4-digit PIN. Have the client read out or send you their 9-digit ID.", "Step 1: On the Client System (To Be Controlled)");
doc.addBullet("Launch RemoteDesk Free on your computer. In the right card ('Control Remote Desk'), enter the client's 9-digit ID into the Target Desk ID field.", "Step 2: On Your System (The Controller)");
doc.addBullet("Click the blue 'Connect' button. Select which permissions you wish to request (Screen View, Mouse/Keyboard, Clipboard, File Transfer).", "Step 3: Initiate Connection");
doc.addBullet("On the client's computer, an incoming prompt will appear showing your partner ID. Once they click 'Accept' (or if you enter their PIN / unattended password), the remote desktop session opens immediately!", "Step 4: Authorization");

// 5. Section 4: In-Session Remote Desktop & Controls (With Screenshots)
doc.addSectionHeader("4. In-Session Remote Desktop, Toolbar & Controls", true);
doc.addParagraph("When connected, the full-screen interactive remote viewer provides instantaneous screen rendering, mouse and keyboard injection, and live telemetry.");

doc.addImage('RemoteSession', 185, 'Live Interactive Remote Desktop View with Windows 10/11 Desktop, Start Menu & Apps');

doc.addSubHeader("Live Telemetry & Performance Indicators");
doc.addParagraph("The viewer header features live real-time network and rendering telemetry badges:");
doc.addBullet("Displays actual decoded framerate (30–60 FPS). Reflects delta updates and smooth animations.", "Framerate (FPS)");
doc.addBullet("Shows round-trip network latency (< 15 ms on local/regional networks, sub-30ms over WebRTC UDP).", "Latency (ms)");
doc.addBullet("Indicates streaming transport mode ('WebRTC P2P (Live)' for direct UDP peer-to-peer or 'Global Mesh' for cloud rendezvous) along with exact remote resolution (e.g. 1366x768, 1920x1080).", "Transport & Resolution");

doc.addImage('ToolbarBadges', 75, 'Compact Non-Wrapping In-Session Toolbar with Instant System Action Buttons');

doc.addSubHeader("⚡ Instant Quick Action Buttons");
doc.addBullet("Opens the Windows Run dialog (Win + R) on the remote machine with physical scan code simulation.", "▶️ Run");
doc.addBullet("Immediately launches Task Manager (Ctrl + Shift + Esc) on the remote computer.", "💻 TaskMgr");
doc.addBullet("Toggles the remote Windows Start Menu.", "🪟 Win");
doc.addBullet("Green active badge indicating all keyboard shortcuts (Win+R, Win+E, Win+D, Alt+Tab, Alt+F4) pass directly to the remote PC without local interception.", "⌨️ Remote Passthrough");

doc.addSubHeader("⚡ Actions ▾ Dropdown Menu Reference");
doc.addTable(
    ["Toolbar Action", "Win32 Injection Sequence", "Operational Function"],
    [
        ["▶️ Run Dialog", "VK_LWIN + VK_R (Scan 0x13)", "Opens Run dialog to execute cmd, powershell, regedit"],
        ["💻 Task Manager", "VK_CONTROL + VK_SHIFT + VK_ESCAPE", "Launches Task Manager process & performance view"],
        ["📁 File Explorer", "VK_LWIN + VK_E (Scan 0x12)", "Opens File Explorer on remote computer"],
        ["🖥️ Show Desktop", "VK_LWIN + VK_D (Scan 0x20)", "Minimizes all windows to display desktop"],
        ["⚡ Quick Link Menu", "VK_LWIN + VK_X (Scan 0x2D)", "Opens Windows power user admin menu"],
        ["⚙️ Windows Settings", "VK_LWIN + VK_I (Scan 0x17)", "Opens Windows Settings / Control Panel"],
        ["🔀 Alt + Tab", "VK_MENU + VK_TAB (Dwell 20ms)", "Switches active application window"],
        ["❌ Close Window (Alt+F4)", "VK_MENU + VK_F4 (Scan 0x3E)", "Closes active foreground window on remote PC"],
        ["🔒 Lock Workstation", "VK_LWIN + VK_L (Scan 0x26)", "Locks remote workstation and displays login screen"],
        ["⌨️ Send Ctrl + Alt + Del", "Ctrl + Alt + Del SAS Injection", "Sends secure attention sequence to remote Windows"],
        ["📋 Paste Clipboard Text", "Unicode Stream Injection", "Transfers and types multi-line text directly into remote app"]
    ],
    [125, 165, 222]
);

// 6. Section 5: Display Sizing & Responsive Canvas Fit
doc.addSectionHeader("5. Display Scaling, Aspect Ratio & Responsive Fit", true);
doc.addParagraph("RemoteDesk Free includes a dedicated aspect ratio engine that automatically scales the remote screen to fit your window while preserving perfect 1:1 mouse coordinate alignment.");

doc.addSubHeader("Scaling Selector Options (Toolbar Dropdown)");
doc.addBullet("Proportionally scales the remote desktop to fill maximum available window space while preserving the remote computer's native aspect ratio (e.g. 16:9, 16:10, 4:3). Default and recommended mode.", "Scale to Fit (Default)");
doc.addBullet("Renders remote screen at native 100% pixel resolution. If the remote screen is larger than your monitor, smooth scrollbars allow panning.", "Original (1:1)");
doc.addBullet("Stretches the remote video canvas to fill the entire window area without borders.", "Stretch");

doc.addCallout("Sub-Pixel Coordinate Normalization", "Mouse coordinates are dynamically calculated using getBoundingClientRect() and mapped to the remote workstation's exact native resolution, ensuring that clicks, double-clicks, and dragging hit the exact intended pixel regardless of window scaling.", "success");

// 7. Section 6: Dual-Pane Cryptographic File Manager
doc.addSectionHeader("6. Dual-Pane Cryptographic File Transfer & Manager", true);
doc.addParagraph("Clicking the '📁 Files' button on the toolbar opens an enterprise-grade dual-pane file transfer interface designed for rapid, secure deployment of tools, scripts, and logs.");

doc.addSubHeader("File Transfer Engine Highlights");
doc.addBullet("Side-by-side view showing Local Machine (left pane) and Remote Desktop (right pane).", "Dual-Pane Layout");
doc.addBullet("1-click navigation buttons for Downloads, Desktop, and Root Drive (C:\\) on both local and remote file systems.", "Quick Directory Navigation");
doc.addBullet("Files are transferred in 64 KB binary chunks to prevent memory spikes and ensure resilience over intermittent links.", "64 KB Chunk Streaming");
doc.addBullet("Every transferred file undergoes pre-send and post-receive SHA-256 cryptographic hash verification to ensure 100% bit-for-bit file integrity.", "SHA-256 Checksum Verification");
doc.addBullet("Click '📤 Direct Upload Any File...' to browse and upload any file from your computer directly into the active remote directory.", "Direct File Upload");

// 8. Section 7: Multi-Session Management & Background Minimizing
doc.addSectionHeader("7. Multi-Session Tabs & Background Minimization", true);
doc.addParagraph("IT administrators frequently manage multiple client systems simultaneously. RemoteDesk Free natively supports concurrent multi-session tabs.");

doc.addSubHeader("Managing Multiple Client Systems");
doc.addBullet("Click '+ Add' in the session toolbar to connect to another remote machine without disconnecting the current session.", "Concurrent Connections");
doc.addBullet("Each connected workstation appears as an interactive tab (e.g. '🖥️ 660 856 593'). Click any tab to switch between client desktops instantaneously.", "Interactive Tabs Bar");
doc.addBullet("Click '🗕 Min' on the toolbar to dock active sessions into a slim dashboard banner ('Active Remote Sessions'). The session continues streaming in the background without dropping.", "Zero-Disruption Minimize");
doc.addBullet("Click 'Resume Session' from the dashboard to return instantly to your remote desktop with immediate bitmap repainting and zero delay.", "Instant Resumption");

// 9. Section 8: Address Book, Saved Devices & Recent History
doc.addSectionHeader("8. Address Book, Saved Devices & Recent History", true);
doc.addParagraph("The Address Book (Tab 3) and Recent Connections panel provide full lifecycle organization for your managed infrastructure.");

doc.addSubHeader("Address Book Capabilities");
doc.addBullet("Register computers with 9-digit Device ID, friendly computer name (e.g. 'Accounting-PC-01'), and optional quick-connect credentials.", "Device Registration");
doc.addBullet("Organize infrastructure into Work, Servers, Clients, Support, and Personal categories.", "Group Categorization");
doc.addBullet("Search bar filters through IDs, hostnames, and group tags instantaneously as you type.", "Instant Search");
doc.addBullet("Every connection is logged on the dashboard with relative timestamps ('Just now', '15m ago') and a 1-click '⚡ Connect' button.", "Recent Connections History");
doc.addBullet("Click the '⭐' icon on any recent connection row to instantly save that machine into your Address Book.", "1-Click Bookmarking");

// 10. Section 9: Security, Unattended Access & Audit Trail
doc.addSectionHeader("9. Unattended Access, Security & Audit Logging", true);
doc.addParagraph("For 24/7 server and office PC administration, RemoteDesk Free includes secure unattended access protected by salted PBKDF2/SHA-256 password hashing.");

doc.addSubHeader("Unattended IT Administration");
doc.addBullet("On the Security tab, enable 'Unattended Access' and set a master password (minimum 8 characters).", "Configuration");
doc.addBullet("Passwords are hashed locally with PBKDF2 (100,000 iterations and cryptographic salt). Plain-text passwords are NEVER stored on disk or transmitted over the network.", "Salted Hashing");
doc.addBullet("Authorized administrators can connect at any time using the master password without requiring a person to be physically present at the remote computer.", "24/7 Zero-Touch Access");
doc.addBullet("Every session start, permission grant, interactive acceptance, and unattended login is recorded in a local tamper-evident audit log ('audit.json').", "Local Audit Trail");

// 11. Section 10: Antivirus Verification & Heuristic Resolution (With Screenshot)
doc.addSectionHeader("10. Antivirus Verification & Heuristic Resolution", true);
doc.addParagraph("Because RemoteDesk Free incorporates low-level Win32 input injection (SendInput) and screen capture (BitBlt) APIs, automated cloud machine learning models (such as Windows Defender Trojan:Win32/Wacatac.B!ml or SecureAge) may initially flag unsigned builds as suspicious.");

doc.addImage('SecurityUI', 180, 'Windows Security & Antivirus Heuristic Evaluation Interface');

doc.addCallout("Understanding the '!ml' Heuristic Suffix", "The '!ml' suffix in Microsoft Defender stands for 'Machine Learning'. It signifies an automated algorithmic heuristic score applied to new or freshly compiled binaries, NOT an actual Trojan signature or malware match. Any newly compiled binary that contains input automation APIs receives this temporary evaluation until reputation accumulates.", "warning");

doc.addSubHeader("Built-In Verifications in v1.1.0.1:");
doc.addBullet("Includes embedded Windows 10/11 compatibility GUIDs and non-elevated user execution privilege (<requestedExecutionLevel level='asInvoker'/>).", "Embedded Application Manifest");
doc.addBullet("Contains verified Windows PE VersionInfo attributes (FileVersion: 1.1.0.1, Company: Freeware | Suresh Mamidala, LegalCopyright: Copyright (C) 2026 Suresh Mamidala).", "PE VersionInfo Resources");
doc.addBullet("Configured with non-solid transparent compression (SolidCompression=no) so antivirus scanners can verify internal clean binaries directly.", "Transparent Inno Setup Packaging");

// 12. Section 11: Technical Specifications & Publisher Reference
doc.addSectionHeader("11. Technical Specifications & Publication Metadata", true);

doc.addTable(
    ["Attribute", "Specification / Value", "Corporate Details"],
    [
        ["Application Name", "RemoteDesk Free", "Windows Enterprise Remote Desktop"],
        ["File Version", "1.1.0.1", "Production Release"],
        ["Company Name", "Freeware | Suresh Mamidala", "Enterprise Open Freeware"],
        ["Publisher / Author", "Suresh Mamidala", "Software Architecture & Security"],
        ["Legal Copyright", "Copyright (C) 2026 Suresh Mamidala", "All Rights Reserved"],
        ["Default Install Path", "C:\\Program Files\\RemoteDesk Free", "Standard 64-bit Program Files"],
        ["Supported OS", "Windows 10 / Windows 11", "x64 & ARM64 Architecture"],
        ["Transport Protocols", "WebRTC (UDP) / MQTT over WSS (TCP 443)", "Sub-20ms P2P with Cloud Mesh Relay"],
        ["Encryption", "DTLS-SRTP / TLS 1.3 / PBKDF2 SHA-256", "Military-grade End-to-End Encryption"]
    ],
    [125, 175, 212]
);

doc.addParagraph("For technical assistance, corporate deployment questions, or feature feedback, contact the development and publishing team at Freeware | Suresh Mamidala.");

// Compile PDF
const pdfBuffer = doc.compile();
const outPath = path.join(__dirname, 'RemoteDesk-Free-User-Guide.pdf');
fs.writeFileSync(outPath, pdfBuffer);

console.log(`Successfully generated ${outPath} (${pdfBuffer.length} bytes, ${doc.totalPages} pages)`);
