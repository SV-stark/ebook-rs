/// Embedded modern HTML/CSS/JS reader web application interface with live DOM Selection-to-CFI converter,
/// double-page spread engine, and continuous vertical scroll (`scrolled-doc`).
pub const READER_HTML: &str = r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <style>
        :root {
            --bg-primary: #0f172a;
            --bg-surface: #1e293b;
            --bg-hover: #334155;
            --accent: #3b82f6;
            --accent-hover: #2563eb;
            --text-primary: #f8fafc;
            --text-secondary: #94a3b8;
            --border-color: #334155;
            --sidebar-width: 340px;
        }

        body.theme-light {
            --bg-primary: #f8fafc;
            --bg-surface: #ffffff;
            --bg-hover: #f1f5f9;
            --accent: #2563eb;
            --accent-hover: #1d4ed8;
            --text-primary: #0f172a;
            --text-secondary: #64748b;
            --border-color: #e2e8f0;
        }

        body.theme-sepia {
            --bg-primary: #fef3c7;
            --bg-surface: #fde68a;
            --bg-hover: #fcd34d;
            --accent: #d97706;
            --accent-hover: #b45309;
            --text-primary: #451a03;
            --text-secondary: #78350f;
            --border-color: #f59e0b;
        }

        body.theme-midnight {
            --bg-primary: #000000;
            --bg-surface: #0a0a0c;
            --bg-hover: #18181b;
            --accent: #38bdf8;
            --accent-hover: #0284c7;
            --text-primary: #f1f5f9;
            --text-secondary: #a1a1aa;
            --border-color: #27272a;
        }

        * { box-sizing: border-box; margin: 0; padding: 0; }

        body {
            font-family: system-ui, -apple-system, 'Segoe UI', Roboto, 'Inter', sans-serif;
            background-color: var(--bg-primary);
            color: var(--text-primary);
            height: 100vh;
            display: flex;
            flex-direction: column;
            overflow: hidden;
            transition: background 0.2s ease, color 0.2s ease;
        }

        /* Top Navbar */
        header {
            height: 56px;
            background: var(--bg-surface);
            border-bottom: 1px solid var(--border-color);
            display: flex;
            align-items: center;
            justify-content: space-between;
            padding: 0 16px;
            z-index: 20;
        }

        .nav-left, .nav-right {
            display: flex;
            align-items: center;
            gap: 10px;
        }

        .book-title {
            font-weight: 600;
            font-size: 0.95rem;
            max-width: 320px;
            white-space: nowrap;
            overflow: hidden;
            text-overflow: ellipsis;
        }

        .btn {
            background: var(--bg-hover);
            color: var(--text-primary);
            border: 1px solid var(--border-color);
            padding: 6px 12px;
            border-radius: 6px;
            cursor: pointer;
            font-size: 0.85rem;
            font-weight: 500;
            display: inline-flex;
            align-items: center;
            gap: 6px;
            transition: all 0.15s ease;
        }

        .btn:hover {
            background: var(--accent);
            color: #ffffff;
            border-color: var(--accent);
        }

        .btn.active {
            background: var(--accent);
            color: #ffffff;
            border-color: var(--accent);
        }

        /* Container & Sidebar Layout */
        .main-container {
            flex: 1;
            display: flex;
            position: relative;
            overflow: hidden;
        }

        sidebar {
            width: var(--sidebar-width);
            background: var(--bg-surface);
            border-right: 1px solid var(--border-color);
            display: flex;
            flex-direction: column;
            position: absolute;
            top: 0; bottom: 0; left: 0;
            z-index: 15;
            transform: translateX(-100%);
            transition: transform 0.25s cubic-bezier(0.4, 0, 0.2, 1);
        }

        sidebar.open {
            transform: translateX(0);
        }

        .sidebar-tabs {
            display: flex;
            border-bottom: 1px solid var(--border-color);
        }

        .tab-btn {
            flex: 1;
            padding: 12px 8px;
            background: transparent;
            border: none;
            color: var(--text-secondary);
            font-weight: 600;
            font-size: 0.8rem;
            cursor: pointer;
            border-bottom: 2px solid transparent;
        }

        .tab-btn.active {
            color: var(--accent);
            border-bottom-color: var(--accent);
        }

        .tab-content {
            flex: 1;
            overflow-y: auto;
            padding: 16px;
            display: none;
        }

        .tab-content.active {
            display: block;
        }

        .toc-item {
            padding: 8px 12px;
            border-radius: 6px;
            cursor: pointer;
            font-size: 0.9rem;
            margin-bottom: 4px;
            color: var(--text-primary);
            transition: background 0.15s ease;
        }

        .toc-item:hover {
            background: var(--bg-hover);
            color: var(--accent);
        }

        .toc-sub {
            margin-left: 16px;
            border-left: 2px solid var(--border-color);
            padding-left: 8px;
        }

        /* Reader Stage */
        .reader-stage {
            flex: 1;
            display: flex;
            justify-content: center;
            align-items: center;
            position: relative;
            background: var(--bg-primary);
            overflow: hidden;
        }

        .reader-stage.scrolled-mode {
            overflow-y: auto;
            align-items: flex-start;
        }

        iframe#section-frame {
            width: 100%;
            height: 100%;
            border: none;
            background: transparent;
            transition: all 0.2s ease;
        }

        /* Floating Selection Highlight Toolbar */
        .selection-toolbar {
            position: absolute;
            display: none;
            background: var(--bg-surface);
            border: 1px solid var(--border-color);
            padding: 6px;
            border-radius: 8px;
            box-shadow: 0 8px 24px rgba(0,0,0,0.3);
            z-index: 100;
            gap: 6px;
            align-items: center;
        }

        .color-dot {
            width: 20px;
            height: 20px;
            border-radius: 50%;
            cursor: pointer;
            border: 1px solid rgba(255,255,255,0.2);
            transition: transform 0.1s ease;
        }

        .color-dot:hover { scale: 1.2; }

        /* Navigation Page Buttons */
        .page-btn {
            position: absolute;
            top: 50%;
            transform: translateY(-50%);
            width: 44px;
            height: 44px;
            border-radius: 50%;
            background: var(--bg-surface);
            border: 1px solid var(--border-color);
            color: var(--text-primary);
            font-size: 1.2rem;
            cursor: pointer;
            display: flex;
            align-items: center;
            justify-content: center;
            z-index: 10;
            box-shadow: 0 4px 12px rgba(0,0,0,0.15);
            transition: all 0.15s ease;
        }

        .page-btn:hover {
            background: var(--accent);
            color: #fff;
            scale: 1.05;
        }

        .page-btn.left { left: 16px; }
        .page-btn.right { right: 16px; }

        /* Footer Status */
        footer {
            height: 44px;
            background: var(--bg-surface);
            border-top: 1px solid var(--border-color);
            display: flex;
            align-items: center;
            justify-content: space-between;
            padding: 0 16px;
            font-size: 0.8rem;
            color: var(--text-secondary);
            z-index: 20;
        }

        .progress-bar-container {
            flex: 1;
            max-width: 400px;
            margin: 0 20px;
            display: flex;
            align-items: center;
            gap: 10px;
        }

        input[type="range"] {
            flex: 1;
            accent-color: var(--accent);
            cursor: pointer;
        }

        .search-box {
            display: flex;
            gap: 8px;
            margin-bottom: 16px;
        }

        .input-text {
            flex: 1;
            background: var(--bg-hover);
            border: 1px solid var(--border-color);
            color: var(--text-primary);
            padding: 8px 12px;
            border-radius: 6px;
            font-size: 0.85rem;
            outline: none;
        }

        .search-result, .ann-item {
            padding: 10px;
            background: var(--bg-hover);
            border-radius: 6px;
            margin-bottom: 8px;
            cursor: pointer;
            font-size: 0.85rem;
        }

        .search-result:hover, .ann-item:hover {
            border: 1px solid var(--accent);
        }

        .cfi-tag {
            font-size: 0.75rem;
            color: var(--accent);
            margin-top: 4px;
            font-family: 'Fira Code', monospace;
        }

        .sidebar-footer {
            padding: 12px 16px;
            border-top: 1px solid var(--border-color);
            display: flex;
            gap: 8px;
            background: var(--bg-surface);
            margin-top: auto;
        }

        /* Modal Dialogs */
        .modal-backdrop {
            position: fixed;
            top: 0; left: 0; right: 0; bottom: 0;
            background: rgba(0, 0, 0, 0.65);
            backdrop-filter: blur(4px);
            z-index: 1000;
            display: none;
            align-items: center;
            justify-content: center;
            padding: 16px;
            opacity: 0;
            transition: opacity 0.2s ease;
        }

        .modal-backdrop.open {
            display: flex;
            opacity: 1;
        }

        .modal-box {
            background: var(--bg-surface);
            color: var(--text-primary);
            border: 1px solid var(--border-color);
            border-radius: 12px;
            width: 100%;
            max-width: 580px;
            max-height: 85vh;
            display: flex;
            flex-direction: column;
            box-shadow: 0 20px 40px rgba(0, 0, 0, 0.4);
            animation: modalSlideUp 0.2s ease forwards;
            overflow: hidden;
        }

        @keyframes modalSlideUp {
            from { transform: translateY(16px); opacity: 0; }
            to { transform: translateY(0); opacity: 1; }
        }

        .modal-header {
            padding: 16px 20px;
            border-bottom: 1px solid var(--border-color);
            display: flex;
            align-items: center;
            justify-content: space-between;
        }

        .modal-title {
            font-size: 1.1rem;
            font-weight: 600;
            display: flex;
            align-items: center;
            gap: 8px;
        }

        .btn-close {
            background: transparent;
            border: none;
            color: var(--text-secondary);
            font-size: 1.4rem;
            line-height: 1;
            cursor: pointer;
            padding: 4px 8px;
            border-radius: 4px;
            transition: all 0.15s ease;
        }

        .btn-close:hover {
            background: var(--bg-hover);
            color: var(--text-primary);
        }

        .modal-body {
            padding: 20px;
            overflow-y: auto;
            flex: 1;
        }

        .modal-footer {
            padding: 12px 20px;
            border-top: 1px solid var(--border-color);
            display: flex;
            justify-content: flex-end;
            gap: 10px;
            background: rgba(0, 0, 0, 0.12);
        }

        .settings-section {
            margin-bottom: 22px;
        }

        .settings-section:last-child {
            margin-bottom: 0;
        }

        .settings-section-title {
            font-size: 0.8rem;
            font-weight: 700;
            text-transform: uppercase;
            letter-spacing: 0.05em;
            color: var(--accent);
            margin-bottom: 12px;
            display: flex;
            align-items: center;
            gap: 6px;
        }

        .setting-row {
            display: flex;
            align-items: center;
            justify-content: space-between;
            margin-bottom: 12px;
            gap: 16px;
        }

        .setting-label {
            font-size: 0.9rem;
            font-weight: 500;
        }

        .setting-desc {
            font-size: 0.78rem;
            color: var(--text-secondary);
            margin-top: 2px;
        }

        .setting-control {
            display: flex;
            align-items: center;
            gap: 8px;
        }

        .select-input {
            background: var(--bg-hover);
            color: var(--text-primary);
            border: 1px solid var(--border-color);
            padding: 6px 12px;
            border-radius: 6px;
            font-size: 0.85rem;
            outline: none;
            cursor: pointer;
        }

        .select-input:focus {
            border-color: var(--accent);
        }

        .about-badge-grid {
            display: flex;
            flex-wrap: wrap;
            gap: 8px;
            margin: 14px 0;
        }

        .badge {
            display: inline-flex;
            align-items: center;
            gap: 4px;
            background: var(--bg-hover);
            color: var(--text-primary);
            border: 1px solid var(--border-color);
            padding: 4px 10px;
            border-radius: 20px;
            font-size: 0.78rem;
            font-weight: 500;
        }

        .badge-accent {
            background: rgba(59, 130, 246, 0.15);
            border-color: var(--accent);
            color: var(--accent);
        }

        .about-info-table {
            width: 100%;
            border-collapse: collapse;
            font-size: 0.85rem;
        }

        .about-info-table td {
            padding: 8px 10px;
            border-bottom: 1px solid var(--border-color);
        }

        .about-info-table td:first-child {
            color: var(--text-secondary);
            width: 32%;
            font-weight: 500;
        }

        .about-shortcuts {
            background: var(--bg-hover);
            border: 1px solid var(--border-color);
            border-radius: 8px;
            padding: 10px 14px;
            font-size: 0.82rem;
        }

        .shortcut-item {
            display: flex;
            justify-content: space-between;
            align-items: center;
            padding: 5px 0;
        }

        .kbd {
            background: var(--bg-surface);
            border: 1px solid var(--border-color);
            border-radius: 4px;
            padding: 2px 6px;
            font-family: monospace;
            font-size: 0.75rem;
            box-shadow: 0 1px 2px rgba(0,0,0,0.2);
        }
    </style>
</head>
<body class="theme-dark">

    <header>
        <div class="nav-left">
            <button class="btn" id="btn-sidebar">☰ Menu</button>
            <div class="book-title" id="lbl-title">Loading Book...</div>
        </div>
        <div class="nav-right">
            <!-- Feature 4: Spread Switcher -->
            <button class="btn active" id="btn-spread-single">📄 Single</button>
            <button class="btn" id="btn-spread-double">📖 Double Spread</button>
            
            <!-- Feature 6: Flow Switcher -->
            <button class="btn active" id="btn-flow-paginated">📑 Paginated</button>
            <button class="btn" id="btn-flow-scrolled">📜 Continuous Scroll</button>
            
            <button class="btn" id="btn-theme-dark">🌙 Dark</button>
            <button class="btn" id="btn-theme-light">☀️ Light</button>
            <button class="btn" id="btn-theme-sepia">📜 Sepia</button>
            <button class="btn" id="btn-font-dec">A-</button>
            <button class="btn" id="btn-font-inc">A+</button>
            <button class="btn" id="btn-open-settings" title="Reader Settings">⚙️ Settings</button>
            <button class="btn" id="btn-open-about" title="About App">ℹ️ About</button>
        </div>
    </header>

    <div class="main-container">
        <sidebar id="sidebar">
            <div class="sidebar-tabs">
                <button class="tab-btn active" onclick="switchTab('tab-toc', event)">Contents</button>
                <button class="tab-btn" onclick="switchTab('tab-search', event)">Search</button>
                <button class="tab-btn" onclick="switchTab('tab-annotations', event)">Annotations</button>
            </div>
            <div id="tab-toc" class="tab-content active">
                <div id="toc-list">Loading contents...</div>
            </div>
            <div id="tab-search" class="tab-content">
                <div class="search-box">
                    <input type="text" id="txt-search" class="input-text" placeholder="Search book text...">
                    <button class="btn" id="btn-search-go">Search</button>
                </div>
                <div id="search-results"></div>
            </div>
            <div id="tab-annotations" class="tab-content">
                <div id="annotations-list">No annotations yet. Select text in book to highlight.</div>
            </div>
            <div class="sidebar-footer">
                <button class="btn" style="flex:1; justify-content:center;" id="btn-sidebar-settings">⚙️ Settings</button>
                <button class="btn" style="flex:1; justify-content:center;" id="btn-sidebar-about">ℹ️ About App</button>
            </div>
        </sidebar>

        <div class="reader-stage" id="reader-stage">
            <button class="page-btn left" id="btn-prev">❮</button>
            <iframe id="section-frame" sandbox="allow-scripts" src="about:blank"></iframe>
            <button class="page-btn right" id="btn-next">❯</button>

            <!-- Feature 2: Floating Highlight Selection Bar -->
            <div class="selection-toolbar" id="selection-toolbar">
                <div class="color-dot" style="background:#fef08a" onclick="addHighlight('#fef08a')"></div>
                <div class="color-dot" style="background:#bbf7d0" onclick="addHighlight('#bbf7d0')"></div>
                <div class="color-dot" style="background:#bfdbfe" onclick="addHighlight('#bfdbfe')"></div>
                <div class="color-dot" style="background:#fbcfe8" onclick="addHighlight('#fbcfe8')"></div>
                <button class="btn" style="padding:2px 6px; font-size:0.75rem;" onclick="addBookmark()">📌 Bookmark</button>
            </div>
        </div>
    </div>

    <!-- Settings Modal -->
    <div class="modal-backdrop" id="modal-settings">
        <div class="modal-box" role="dialog" aria-labelledby="modal-settings-title" aria-modal="true">
            <div class="modal-header">
                <div class="modal-title" id="modal-settings-title">⚙️ Reader Settings</div>
                <button class="btn-close" id="btn-close-settings-x" aria-label="Close settings">&times;</button>
            </div>
            <div class="modal-body">
                <div class="settings-section">
                    <div class="settings-section-title">🎨 Appearance & Theme</div>
                    <div class="setting-row">
                        <div>
                            <div class="setting-label">Color Theme</div>
                            <div class="setting-desc">Reading palette and interface contrast</div>
                        </div>
                        <div class="setting-control">
                            <select id="setting-theme" class="select-input">
                                <option value="theme-dark">🌙 Dark (Default)</option>
                                <option value="theme-light">☀️ Light</option>
                                <option value="theme-sepia">📜 Sepia</option>
                                <option value="theme-midnight">🖤 Midnight (OLED)</option>
                            </select>
                        </div>
                    </div>
                </div>

                <div class="settings-section">
                    <div class="settings-section-title">🔤 Typography</div>
                    <div class="setting-row">
                        <div>
                            <div class="setting-label">Font Family</div>
                            <div class="setting-desc">Typeface used for rendering book text</div>
                        </div>
                        <div class="setting-control">
                            <select id="setting-font-family" class="select-input">
                                <option value="Georgia, 'Times New Roman', serif">Serif (Georgia)</option>
                                <option value="system-ui, -apple-system, 'Segoe UI', Roboto, sans-serif">Sans-Serif (System)</option>
                                <option value="'Merriweather', Palatino, serif">Literary (Merriweather)</option>
                                <option value="'Fira Code', 'Courier New', monospace">Monospace (Fira Code)</option>
                                <option value="'OpenDyslexic', 'Comic Sans MS', sans-serif">Dyslexic-friendly</option>
                            </select>
                        </div>
                    </div>
                    <div class="setting-row">
                        <div>
                            <div class="setting-label">Font Size</div>
                            <div class="setting-desc">Base text size for comfortable reading</div>
                        </div>
                        <div class="setting-control">
                            <input type="range" id="setting-font-size" min="12" max="36" step="1" value="18">
                            <span id="setting-font-size-val" style="min-width: 44px; text-align: right; font-weight: 600;">18px</span>
                        </div>
                    </div>
                    <div class="setting-row">
                        <div>
                            <div class="setting-label">Line Spacing</div>
                            <div class="setting-desc">Vertical height between lines of text</div>
                        </div>
                        <div class="setting-control">
                            <select id="setting-line-height" class="select-input">
                                <option value="1.4">Compact (1.4)</option>
                                <option value="1.7">Normal (1.7)</option>
                                <option value="2.0">Relaxed (2.0)</option>
                                <option value="2.3">Spacious (2.3)</option>
                            </select>
                        </div>
                    </div>
                    <div class="setting-row">
                        <div>
                            <div class="setting-label">Text Alignment</div>
                            <div class="setting-desc">Justify or left-align paragraph text</div>
                        </div>
                        <div class="setting-control">
                            <select id="setting-text-align" class="select-input">
                                <option value="justify">Justified</option>
                                <option value="left">Left Aligned</option>
                            </select>
                        </div>
                    </div>
                </div>

                <div class="settings-section">
                    <div class="settings-section-title">📐 Layout & Dimensions</div>
                    <div class="setting-row">
                        <div>
                            <div class="setting-label">Spread Mode</div>
                            <div class="setting-desc">Single page view or 2-page open book spread</div>
                        </div>
                        <div class="setting-control">
                            <select id="setting-spread" class="select-input">
                                <option value="single">Single Page</option>
                                <option value="double">2-Page Double Spread</option>
                            </select>
                        </div>
                    </div>
                    <div class="setting-row">
                        <div>
                            <div class="setting-label">Flow Mode</div>
                            <div class="setting-desc">Paginated page turns or vertical scroll</div>
                        </div>
                        <div class="setting-control">
                            <select id="setting-flow" class="select-input">
                                <option value="paginated">Paginated</option>
                                <option value="scrolled">Continuous Scroll</option>
                            </select>
                        </div>
                    </div>
                    <div class="setting-row">
                        <div>
                            <div class="setting-label">Reading Column Width</div>
                            <div class="setting-desc">Maximum width of reading text column</div>
                        </div>
                        <div class="setting-control">
                            <select id="setting-max-width" class="select-input">
                                <option value="650">Narrow (650px)</option>
                                <option value="800">Standard (800px)</option>
                                <option value="1050">Wide (1050px)</option>
                                <option value="100%">Full Width (100%)</option>
                            </select>
                        </div>
                    </div>
                    <div class="setting-row">
                        <div>
                            <div class="setting-label">Page Margins</div>
                            <div class="setting-desc">Inner padding around book content</div>
                        </div>
                        <div class="setting-control">
                            <select id="setting-padding" class="select-input">
                                <option value="20">Compact (20px)</option>
                                <option value="32">Normal (32px)</option>
                                <option value="48">Spacious (48px)</option>
                            </select>
                        </div>
                    </div>
                </div>
            </div>
            <div class="modal-footer">
                <button class="btn" id="btn-reset-settings">↺ Reset to Defaults</button>
                <button class="btn active" id="btn-close-settings">Done</button>
            </div>
        </div>
    </div>

    <!-- About App Modal -->
    <div class="modal-backdrop" id="modal-about">
        <div class="modal-box" role="dialog" aria-labelledby="modal-about-title" aria-modal="true">
            <div class="modal-header">
                <div class="modal-title" id="modal-about-title">ℹ️ About ebook-rs</div>
                <button class="btn-close" id="btn-close-about-x" aria-label="Close about">&times;</button>
            </div>
            <div class="modal-body">
                <div style="text-align: center; margin-bottom: 16px;">
                    <div style="font-size: 1.6rem; font-weight: 700; display: inline-flex; align-items: center; gap: 8px;">
                        📖 ebook-rs
                        <span id="about-version" class="badge badge-accent">v0.17.3</span>
                    </div>
                    <p id="about-description" style="color: var(--text-secondary); font-size: 0.85rem; margin-top: 6px;">
                        Pure Rust multi-format eBook engine & modern web reader
                    </p>
                </div>

                <div class="about-badge-grid" style="justify-content: center;">
                    <span class="badge">🦀 100% Pure Rust</span>
                    <span class="badge">⚡ Zero-Copy Parser</span>
                    <span class="badge">📚 13+ Formats</span>
                    <span class="badge">🎯 Readium CFI & LCP</span>
                    <span class="badge">🤖 Native MCP Server</span>
                    <span class="badge">🧠 AI RAG BM25</span>
                    <span class="badge">📜 Continuous & Spread</span>
                </div>

                <div class="settings-section" style="margin-top: 16px;">
                    <div class="settings-section-title">📖 Active Book Information</div>
                    <table class="about-info-table">
                        <tbody>
                            <tr>
                                <td>Title</td>
                                <td id="about-book-title" style="font-weight: 600;">Loading...</td>
                            </tr>
                            <tr>
                                <td>Author(s)</td>
                                <td id="about-book-author">Loading...</td>
                            </tr>
                            <tr>
                                <td>Identifier</td>
                                <td id="about-book-id" style="font-family: monospace; font-size: 0.8rem;">Loading...</td>
                            </tr>
                            <tr>
                                <td>Spine Sections</td>
                                <td id="about-book-spine">1 items</td>
                            </tr>
                        </tbody>
                    </table>
                </div>

                <div class="settings-section">
                    <div class="settings-section-title">📂 Supported eBook Formats</div>
                    <div style="display: flex; flex-wrap: wrap; gap: 6px; font-size: 0.78rem;">
                        <span class="badge">EPUB 2 / EPUB 3 (.epub)</span>
                        <span class="badge">Kindle MOBI (.mobi)</span>
                        <span class="badge">Kindle AZW / AZW3 (.azw3)</span>
                        <span class="badge">Kindle KFX (.kfx)</span>
                        <span class="badge">FictionBook (.fb2)</span>
                        <span class="badge">Comic Archives (.cbz)</span>
                        <span class="badge">PDF Documents (.pdf)</span>
                        <span class="badge">Microsoft Word (.docx)</span>
                        <span class="badge">OpenDocument (.odt)</span>
                        <span class="badge">Rich Text (.rtf)</span>
                        <span class="badge">Text & Markdown (.txt, .md)</span>
                        <span class="badge">Microsoft Reader (.lit)</span>
                    </div>
                </div>

                <div class="settings-section">
                    <div class="settings-section-title">⌨️ Keyboard Navigation Shortcuts</div>
                    <div class="about-shortcuts">
                        <div class="shortcut-item">
                            <span>Next page / Scroll down</span>
                            <span><span class="kbd">→</span> or <span class="kbd">Space</span></span>
                        </div>
                        <div class="shortcut-item">
                            <span>Previous page / Scroll up</span>
                            <span><span class="kbd">←</span></span>
                        </div>
                        <div class="shortcut-item">
                            <span>Close modals / Close menu</span>
                            <span><span class="kbd">Esc</span></span>
                        </div>
                        <div class="shortcut-item">
                            <span>Create Highlight / Bookmark</span>
                            <span><span class="kbd">Select Text</span></span>
                        </div>
                    </div>
                </div>

                <div class="settings-section" style="margin-bottom: 0;">
                    <div class="settings-section-title">🔗 Project & License</div>
                    <table class="about-info-table">
                        <tbody>
                            <tr>
                                <td>Author</td>
                                <td>SV-Stark</td>
                            </tr>
                            <tr>
                                <td>License</td>
                                <td>MIT License</td>
                            </tr>
                            <tr>
                                <td>Repository</td>
                                <td><a href="https://github.com/SV-stark/ebook-rs" target="_blank" rel="noopener noreferrer" style="color: var(--accent); text-decoration: none;">github.com/SV-stark/ebook-rs</a></td>
                            </tr>
                        </tbody>
                    </table>
                </div>
            </div>
            <div class="modal-footer">
                <button class="btn active" id="btn-close-about">Close</button>
            </div>
        </div>
    </div>

    <footer>
        <div id="lbl-location">Section 1 / 1</div>
        <div class="progress-bar-container">
            <span id="lbl-progress-val">0%</span>
            <input type="range" id="slider-progress" min="0" max="100" value="0">
        </div>
        <div id="lbl-cfi" style="font-family: 'Fira Code', monospace; font-size: 0.75rem;">epubcfi(/6/2!/4)</div>
    </footer>

    <script>
        let currentSpineIndex = 0;
        let totalSpineSections = 1;
        let fontSize = 18;
        let isDoubleSpread = false;
        let isScrolledFlow = false;
        let selectedCfiRange = "";
        let selectedText = "";

        const defaultSettings = {
            theme: 'theme-dark',
            fontSize: 18,
            fontFamily: "Georgia, 'Times New Roman', serif",
            lineHeight: 1.7,
            textAlign: 'justify',
            maxWidth: '800',
            pagePadding: 32,
            spread: 'single',
            flow: 'paginated'
        };

        let appSettings = Object.assign({}, defaultSettings);

        function loadSettings() {
            try {
                const s = localStorage.getItem('ebook_rs_settings');
                if (s) {
                    const parsed = JSON.parse(s);
                    appSettings = Object.assign({}, defaultSettings, parsed);
                }
            } catch (e) {
                console.warn('Could not read settings from localStorage', e);
            }
        }

        function saveSettings() {
            try {
                localStorage.setItem('ebook_rs_settings', JSON.stringify(appSettings));
            } catch (e) {
                console.warn('Could not save settings to localStorage', e);
            }
        }

        function syncSettingsControls() {
            const setVal = (id, val) => {
                const el = document.getElementById(id);
                if (el && val !== undefined) el.value = val;
            };
            setVal('setting-theme', appSettings.theme);
            setVal('setting-font-family', appSettings.fontFamily);
            setVal('setting-font-size', appSettings.fontSize);
            const sizeLabel = document.getElementById('setting-font-size-val');
            if (sizeLabel) sizeLabel.innerText = `${appSettings.fontSize}px`;
            setVal('setting-line-height', String(appSettings.lineHeight));
            setVal('setting-text-align', appSettings.textAlign);
            setVal('setting-spread', appSettings.spread);
            setVal('setting-flow', appSettings.flow);
            setVal('setting-max-width', String(appSettings.maxWidth));
            setVal('setting-padding', String(appSettings.pagePadding));
        }

        function applySettings(newSettings, reload = true) {
            appSettings = Object.assign({}, appSettings, newSettings);
            saveSettings();

            fontSize = Number(appSettings.fontSize) || 18;
            isDoubleSpread = (appSettings.spread === 'double');
            isScrolledFlow = (appSettings.flow === 'scrolled');

            document.body.className = appSettings.theme || 'theme-dark';

            const btnSingle = document.getElementById('btn-spread-single');
            const btnDouble = document.getElementById('btn-spread-double');
            if (btnSingle && btnDouble) {
                btnSingle.classList.toggle('active', !isDoubleSpread);
                btnDouble.classList.toggle('active', isDoubleSpread);
            }

            const btnPaginated = document.getElementById('btn-flow-paginated');
            const btnScrolled = document.getElementById('btn-flow-scrolled');
            if (btnPaginated && btnScrolled) {
                btnPaginated.classList.toggle('active', !isScrolledFlow);
                btnScrolled.classList.toggle('active', isScrolledFlow);
            }
            const stage = document.getElementById('reader-stage');
            if (stage) {
                stage.classList.toggle('scrolled-mode', isScrolledFlow);
            }

            syncSettingsControls();

            if (reload) {
                loadSection(currentSpineIndex);
            }
        }

        function openModal(id) {
            closeAllModals();
            const modal = document.getElementById(id);
            if (modal) modal.classList.add('open');
        }

        function closeAllModals() {
            document.querySelectorAll('.modal-backdrop').forEach(m => m.classList.remove('open'));
        }

        let appInfoLoaded = false;
        async function openAboutModal() {
            openModal('modal-about');
            if (!appInfoLoaded) {
                try {
                    const res = await fetch('/api/app/info');
                    if (res.ok) {
                        const info = await res.json();
                        if (info.version) {
                            const vEl = document.getElementById('about-version');
                            if (vEl) vEl.innerText = `v${info.version}`;
                        }
                        if (info.description) {
                            const dEl = document.getElementById('about-description');
                            if (dEl) dEl.innerText = info.description;
                        }
                        appInfoLoaded = true;
                    }
                } catch (e) {
                    console.warn("Could not load /api/app/info", e);
                }
            }
        }

        const sidebar = document.getElementById('sidebar');
        const sectionFrame = document.getElementById('section-frame');
        const selectionToolbar = document.getElementById('selection-toolbar');

        document.getElementById('btn-sidebar').addEventListener('click', () => {
            sidebar.classList.toggle('open');
        });

        function switchTab(tabId, ev) {
            document.querySelectorAll('.tab-btn').forEach(b => b.classList.remove('active'));
            document.querySelectorAll('.tab-content').forEach(c => c.classList.remove('active'));
            // `event.target` (the legacy global) only exists in Chromium; in Firefox and Safari
            // `event` is undefined here and the handler threw, leaving every sidebar panel
            // unusable. Take the event as an explicit argument instead.
            if (ev && ev.target) ev.target.classList.add('active');
            document.getElementById(tabId).classList.add('active');
            if (tabId === 'tab-annotations') refreshAnnotations();
        }

        async function initReader() {
            try {
                loadSettings();
                applySettings({}, false);

                const metaRes = await fetch('/api/book/metadata');
                const meta = await metaRes.json();
                document.getElementById('lbl-title').innerText = meta.title || "EPUB Book";

                const abTitle = document.getElementById('about-book-title');
                if (abTitle) abTitle.innerText = meta.title || "Untitled";
                const abAuthor = document.getElementById('about-book-author');
                if (abAuthor) abAuthor.innerText = (meta.creators && meta.creators.length > 0) ? meta.creators.join(', ') : "Unknown";
                const abId = document.getElementById('about-book-id');
                if (abId) abId.innerText = meta.identifier || "N/A";

                const spineRes = await fetch('/api/book/spine');
                const spine = await spineRes.json();
                totalSpineSections = spine.length;
                const abSpine = document.getElementById('about-book-spine');
                if (abSpine) abSpine.innerText = `${spine.length} items`;

                const tocRes = await fetch('/api/book/toc');
                const toc = await tocRes.json();
                renderToc(toc);

                loadSection(0);
            } catch (err) {
                console.error("Initialization failed:", err);
            }
        }

        // Depth-limited: a maliciously deep or self-referential TOC would otherwise blow the
        // browser stack. `depth` increases strictly, so the recursion always terminates.
        const MAX_TOC_DEPTH = 32;
        function renderToc(items, container = document.getElementById('toc-list'), depth = 0) {
            if (!Array.isArray(items) || depth > MAX_TOC_DEPTH) return;
            container.innerHTML = '';
            items.forEach(item => {
                if (!item || typeof item !== 'object') return;
                const div = document.createElement('div');
                div.className = 'toc-item';
                div.innerText = String(item.label ?? '');
                div.onclick = () => {
                    if (item.href) loadSectionByHref(item.href);
                };
                container.appendChild(div);
                if (Array.isArray(item.subitems) && item.subitems.length > 0) {
                    const subContainer = document.createElement('div');
                    subContainer.className = 'toc-sub';
                    renderToc(item.subitems, subContainer, depth + 1);
                    container.appendChild(subContainer);
                }
            });
        }

        const sectionCache = new Map();
        const MAX_SECTION_CACHE = 24;

        async function getSectionHtml(index) {
            if (sectionCache.has(index)) {
                const val = sectionCache.get(index);
                sectionCache.delete(index);
                sectionCache.set(index, val);
                return val;
            }
            try {
                const res = await fetch(`/api/book/section/${index}`);
                if (!res.ok) return '';
                const html = await res.text();
                if (sectionCache.size >= MAX_SECTION_CACHE) {
                    const oldestKey = sectionCache.keys().next().value;
                    sectionCache.delete(oldestKey);
                }
                sectionCache.set(index, html);
                return html;
            } catch (err) {
                console.error("Fetch section error:", err);
                return '';
            }
        }

        function prefetchAdjacent(index) {
            if (index + 1 < totalSpineSections && !sectionCache.has(index + 1)) {
                getSectionHtml(index + 1);
            }
            if (index - 1 >= 0 && !sectionCache.has(index - 1)) {
                getSectionHtml(index - 1);
            }
        }

        async function loadSection(index, scrollToBottom = false) {
            if (index < 0 || index >= totalSpineSections) return;
            currentSpineIndex = index;

            const theme = appSettings.theme || 'theme-dark';
            const isLight = theme === 'theme-light';
            const isSepia = theme === 'theme-sepia';
            const isMidnight = theme === 'theme-midnight';
            const fgColor = isLight ? '#0f172a' : (isSepia ? '#451a03' : (isMidnight ? '#f1f5f9' : '#f8fafc'));
            const pageBg = isLight ? '#ffffff' : (isSepia ? '#fbf0d9' : (isMidnight ? '#000000' : '#1e293b'));
            const borderCol = isLight ? '#e2e8f0' : (isSepia ? '#e7d8b8' : (isMidnight ? '#27272a' : '#334155'));

            const fontFamily = appSettings.fontFamily || "Georgia, 'Times New Roman', serif";
            const currentFontSize = Number(appSettings.fontSize) || fontSize || 18;
            const lineHeight = appSettings.lineHeight || 1.7;
            const textAlign = appSettings.textAlign || 'justify';
            const maxWidth = appSettings.maxWidth === '100%' ? '100%' : (typeof appSettings.maxWidth === 'number' ? `${appSettings.maxWidth}px` : (appSettings.maxWidth ? (String(appSettings.maxWidth).endsWith('px') ? appSettings.maxWidth : `${appSettings.maxWidth}px`) : '800px'));
            const padding = Number(appSettings.pagePadding) || 32;
            const hPadding = Math.round(padding * 1.5);

            if (isDoubleSpread) {
                // Real 2-Page Open Book Spread (Left Page = Section N, Right Page = Section N+1)
                const htmlLeft = await getSectionHtml(index);
                const hasNext = index + 1 < totalSpineSections;
                const htmlRight = hasNext ? await getSectionHtml(index + 1) : '';
                prefetchAdjacent(hasNext ? index + 1 : index);

                sectionFrame.srcdoc = `
                    <!DOCTYPE html>
                    <html>
                    <head>
                        <meta charset="utf-8">
                        <style>
                            html, body {
                                margin: 0;
                                padding: 0;
                                height: 100vh;
                                overflow: hidden;
                                background: transparent;
                                font-family: ${fontFamily};
                            }
                            .book-spread {
                                display: flex;
                                flex-direction: row;
                                gap: 28px;
                                height: calc(100vh - 40px);
                                max-width: 1400px;
                                margin: 20px auto;
                                padding: 0 24px;
                                box-sizing: border-box;
                            }
                            .book-page {
                                flex: 1;
                                height: 100%;
                                display: flex;
                                flex-direction: column;
                                justify-content: space-between;
                                background: ${pageBg};
                                color: ${fgColor};
                                border-radius: 8px;
                                box-shadow: 0 4px 20px rgba(0, 0, 0, 0.12);
                                padding: ${padding}px ${hPadding}px;
                                box-sizing: border-box;
                                border: 1px solid ${borderCol};
                                overflow: hidden;
                            }
                            .page-content {
                                flex: 1;
                                overflow-y: auto;
                                font-family: ${fontFamily};
                                font-size: ${currentFontSize}px;
                                line-height: ${lineHeight};
                                text-align: ${textAlign};
                                padding-right: 8px;
                            }
                            .page-footer {
                                text-align: center;
                                font-size: 0.75rem;
                                opacity: 0.5;
                                padding-top: 10px;
                                border-top: 1px solid rgba(150, 150, 150, 0.15);
                            }
                            a { color: #3b82f6; }
                            mark { border-radius: 3px; padding: 2px 4px; }
                        </style>
                        <script>
                            document.addEventListener('mouseup', () => {
                                const sel = window.getSelection();
                                if (sel && sel.toString().trim().length > 0) {
                                    const range = sel.getRangeAt(0);
                                    const rect = range.getBoundingClientRect();
                                    window.parent.postMessage({
                                        type: 'selection',
                                        text: sel.toString().trim(),
                                        startOffset: range.startOffset,
                                        endOffset: range.endOffset,
                                        top: rect.top,
                                        left: rect.left,
                                        width: rect.width
                                    }, '*');
                                } else {
                                    window.parent.postMessage({ type: 'selection_clear' }, '*');
                                }
                            });
                        </script>
                    </head>
                    <body>
                        <div class="book-spread">
                            <div class="book-page left-page">
                                <div class="page-content">${htmlLeft}</div>
                                <div class="page-footer">Page ${index + 1} of ${totalSpineSections}</div>
                            </div>
                            <div class="book-page right-page">
                                <div class="page-content">${htmlRight}</div>
                                <div class="page-footer">${hasNext ? 'Page ' + (index + 2) + ' of ' + totalSpineSections : ''}</div>
                            </div>
                        </div>
                    </body>
                    </html>
                `;
            } else {
                // Single Page View
                const html = await getSectionHtml(index);
                prefetchAdjacent(index);

                sectionFrame.srcdoc = `
                    <!DOCTYPE html>
                    <html>
                    <head>
                        <meta charset="utf-8">
                        <style>
                            html, body {
                                margin: 0;
                                padding: 0;
                                background: transparent;
                            }
                            body {
                                font-family: ${fontFamily};
                                font-size: ${currentFontSize}px;
                                line-height: ${lineHeight};
                                text-align: ${textAlign};
                                color: ${fgColor};
                                padding: ${padding}px ${hPadding}px;
                                margin: 0 auto;
                                max-width: ${maxWidth};
                                box-sizing: border-box;
                            }
                            a { color: #3b82f6; }
                            img { max-width: 100%; height: auto; }
                            mark { border-radius: 3px; padding: 2px 4px; }
                        </style>
                        <script>
                            document.addEventListener('mouseup', () => {
                                const sel = window.getSelection();
                                if (sel && sel.toString().trim().length > 0) {
                                    const range = sel.getRangeAt(0);
                                    const rect = range.getBoundingClientRect();
                                    window.parent.postMessage({
                                        type: 'selection',
                                        text: sel.toString().trim(),
                                        startOffset: range.startOffset,
                                        endOffset: range.endOffset,
                                        top: rect.top,
                                        left: rect.left,
                                        width: rect.width
                                    }, '*');
                                } else {
                                    window.parent.postMessage({ type: 'selection_clear' }, '*');
                                }
                            });
                        </script>
                    </head>
                    <body>${html}</body>
                    </html>
                `;
            }

            sectionFrame.onload = () => {
                setupIframeListeners();
                if (scrollToBottom && !isDoubleSpread) {
                    try {
                        const win = sectionFrame.contentWindow;
                        const doc = sectionFrame.contentDocument || win.document;
                        win.scrollTo(0, doc.documentElement.scrollHeight);
                    } catch (e) {}
                }
            };
            updateFooter();
        }

        function goNext() {
            if (isDoubleSpread) {
                if (currentSpineIndex + 2 < totalSpineSections) {
                    loadSection(currentSpineIndex + 2);
                } else if (currentSpineIndex + 1 < totalSpineSections) {
                    loadSection(currentSpineIndex + 1);
                }
            } else {
                try {
                    const win = sectionFrame.contentWindow;
                    const doc = sectionFrame.contentDocument || win.document;
                    if (win && doc && doc.documentElement) {
                        const maxScroll = doc.documentElement.scrollHeight - win.innerHeight;
                        if (win.scrollY < maxScroll - 15) {
                            win.scrollBy({ top: win.innerHeight * 0.85, behavior: 'instant' });
                            return;
                        }
                    }
                } catch (e) {}

                if (currentSpineIndex + 1 < totalSpineSections) {
                    loadSection(currentSpineIndex + 1);
                }
            }
        }

        function goPrev() {
            if (isDoubleSpread) {
                if (currentSpineIndex - 2 >= 0) {
                    loadSection(currentSpineIndex - 2);
                } else if (currentSpineIndex - 1 >= 0) {
                    loadSection(0);
                }
            } else {
                try {
                    const win = sectionFrame.contentWindow;
                    const doc = sectionFrame.contentDocument || win.document;
                    if (win && doc && doc.documentElement) {
                        if (win.scrollY > 15) {
                            win.scrollBy({ top: -win.innerHeight * 0.85, behavior: 'instant' });
                            return;
                        }
                    }
                } catch (e) {}

                if (currentSpineIndex - 1 >= 0) {
                    loadSection(currentSpineIndex - 1, true);
                }
            }
        }

        function computeNodeCfiSubpath(node, offset) {
            let path = [];
            let curr = node;
            while (curr && curr.nodeType !== Node.DOCUMENT_NODE) {
                const parent = curr.parentNode;
                if (!parent) break;
                let elemIdx = 1;
                for (let sibling = parent.firstChild; sibling; sibling = sibling.nextSibling) {
                    if (sibling === curr) break;
                    if (sibling.nodeType === Node.ELEMENT_NODE) elemIdx++;
                }
                path.unshift(elemIdx * 2);
                curr = parent;
            }
            const spineStep = (currentSpineIndex + 1) * 2;
            const sub = path.length > 0 ? '/' + path.join('/') : '/4/2/1';
            return `/6/${spineStep}!${sub}:${offset}`;
        }

        // Feature 2: Secure postMessage DOM Selection to CFI Bridge
        function setupIframeListeners() {
            // Handled via window message event listener
        }

        // The book frame is served with `sandbox="allow-scripts"` (no `allow-same-origin`), which
        // is what makes its untrusted content safe to host. A `srcdoc` frame under that sandbox
        // has an **opaque origin**, so every message it posts arrives with `e.origin === "null"`.
        // Comparing that against the reader's real origin therefore rejects every legitimate
        // message and silently kills text selection, the selection toolbar, `addHighlight()` and
        // every bookmark driven by DOM selection.
        //
        // The authoritative check is window identity below: only a message whose `e.source` is
        // one of *our own* iframes is accepted, and that window reference cannot be forged by any
        // frame we did not create.
        window.addEventListener('message', (e) => {
            const frames = document.querySelectorAll('iframe');
            let fromReaderFrame = false;
            for (const f of frames) {
                if (e.source === f.contentWindow) { fromReaderFrame = true; break; }
            }
            if (!fromReaderFrame) return;

            if (e.data && e.data.type === 'selection') {
                // Coerce to finite numbers: these values are interpolated into inline styles.
                const top_ = Number(e.data.top);
                const left = Number(e.data.left);
                const width = Number(e.data.width);
                if (!Number.isFinite(top_) || !Number.isFinite(left) || !Number.isFinite(width)) return;
                const startOffset = Number(e.data.startOffset);
                const endOffset = Number(e.data.endOffset);
                if (!Number.isFinite(startOffset) || !Number.isFinite(endOffset)) return;

                selectedText = String(e.data.text ?? '');
                const spineStep = (currentSpineIndex + 1) * 2;
                const startCfi = `/6/${spineStep}!/4/2/1:${Math.max(0, Math.trunc(startOffset))}`;
                const endCfi = `/6/${spineStep}!/4/2/1:${Math.max(0, Math.trunc(endOffset))}`;
                selectedCfiRange = `epubcfi(${startCfi},${endCfi})`;

                selectionToolbar.style.display = 'flex';
                selectionToolbar.style.top = `${top_ - 40}px`;
                selectionToolbar.style.left = `${left + (width / 2) - 60}px`;
            } else if (e.data && e.data.type === 'selection_clear') {
                selectionToolbar.style.display = 'none';
            }
        });

        async function addHighlight(color) {
            if (!selectedCfiRange) return;
            await fetch('/api/annotations', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({
                    id: '',
                    cfi_range: selectedCfiRange,
                    type_: 'highlight',
                    color: color,
                    note: 'User highlight',
                    selected_text: selectedText,
                    created_at: `${Math.floor(Date.now()/1000)}`
                })
            });
            selectionToolbar.style.display = 'none';
            refreshAnnotations();
        }

        async function addBookmark() {
            const spineStep = (currentSpineIndex + 1) * 2;
            const cfi = `epubcfi(/6/${spineStep}!/4/2/1:0)`;
            await fetch('/api/annotations', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({
                    id: '',
                    cfi_range: cfi,
                    type_: 'bookmark',
                    color: '#f59e0b',
                    note: `Bookmark Section ${currentSpineIndex + 1}`,
                    selected_text: null,
                    created_at: `${Math.floor(Date.now()/1000)}`
                })
            });
            refreshAnnotations();
        }

        function escapeHtml(str) {
            if (!str) return '';
            return String(str)
                .replace(/&/g, '&amp;')
                .replace(/</g, '&lt;')
                .replace(/>/g, '&gt;')
                .replace(/"/g, '&quot;')
                .replace(/'/g, '&#039;');
        }

        async function refreshAnnotations() {
            const res = await fetch('/api/annotations');
            const list = await res.json();
            const container = document.getElementById('annotations-list');
            if (list.length === 0) {
                container.innerHTML = 'No annotations yet.';
                return;
            }
            container.innerHTML = '';
            list.forEach(a => {
                const div = document.createElement('div');
                div.className = 'ann-item';
                const typeEsc = escapeHtml(a.type_ ? a.type_.toUpperCase() : '');
                const textEsc = escapeHtml(a.selected_text || a.note || '');
                const cfiEsc = escapeHtml(a.cfi_range || '');
                div.innerHTML = `<strong>${typeEsc}</strong>: ${textEsc}<div class="cfi-tag">${cfiEsc}</div>`;
                container.appendChild(div);
            });
        }

        async function loadSectionByHref(href) {
            const cleanHref = href.split('#')[0];
            const spineRes = await fetch('/api/book/spine');
            const spine = await spineRes.json();
            const idx = spine.findIndex(s => s.href.includes(cleanHref) || s.idref === cleanHref);
            if (idx !== -1) loadSection(idx);
            else loadSection(0);
        }

        function updateFooter() {
            document.getElementById('lbl-location').innerText = `Section ${currentSpineIndex + 1} of ${totalSpineSections}`;
            const pct = Math.round(((currentSpineIndex + 1) / totalSpineSections) * 100);
            document.getElementById('lbl-progress-val').innerText = `${pct}%`;
            document.getElementById('slider-progress').value = pct;
            const spineStep = (currentSpineIndex + 1) * 2;
            document.getElementById('lbl-cfi').innerText = `epubcfi(/6/${spineStep}!/4/2/1:0)`;
        }

        // Feature 4: Spread Mode Handlers
        document.getElementById('btn-spread-single').addEventListener('click', () => {
            applySettings({ spread: 'single' });
        });
        document.getElementById('btn-spread-double').addEventListener('click', () => {
            applySettings({ spread: 'double' });
        });

        // Feature 6: Flow Mode Handlers (Paginated vs Scrolled)
        document.getElementById('btn-flow-paginated').addEventListener('click', () => {
            applySettings({ flow: 'paginated' });
        });
        document.getElementById('btn-flow-scrolled').addEventListener('click', () => {
            applySettings({ flow: 'scrolled' });
        });

        document.getElementById('btn-prev').addEventListener('click', goPrev);
        document.getElementById('btn-next').addEventListener('click', goNext);

        document.getElementById('btn-theme-dark').addEventListener('click', () => {
            applySettings({ theme: 'theme-dark' });
        });
        document.getElementById('btn-theme-light').addEventListener('click', () => {
            applySettings({ theme: 'theme-light' });
        });
        document.getElementById('btn-theme-sepia').addEventListener('click', () => {
            applySettings({ theme: 'theme-sepia' });
        });

        document.getElementById('btn-font-inc').addEventListener('click', () => {
            applySettings({ fontSize: Math.min(36, (Number(appSettings.fontSize) || 18) + 2) });
        });
        document.getElementById('btn-font-dec').addEventListener('click', () => {
            applySettings({ fontSize: Math.max(12, (Number(appSettings.fontSize) || 18) - 2) });
        });

        // Modal triggers (Header & Sidebar)
        document.getElementById('btn-open-settings').addEventListener('click', () => openModal('modal-settings'));
        document.getElementById('btn-open-about').addEventListener('click', openAboutModal);
        document.getElementById('btn-sidebar-settings').addEventListener('click', () => {
            sidebar.classList.remove('open');
            openModal('modal-settings');
        });
        document.getElementById('btn-sidebar-about').addEventListener('click', () => {
            sidebar.classList.remove('open');
            openAboutModal();
        });

        document.getElementById('btn-close-settings').addEventListener('click', closeAllModals);
        document.getElementById('btn-close-settings-x').addEventListener('click', closeAllModals);
        document.getElementById('btn-close-about').addEventListener('click', closeAllModals);
        document.getElementById('btn-close-about-x').addEventListener('click', closeAllModals);

        document.querySelectorAll('.modal-backdrop').forEach(b => {
            b.addEventListener('click', (e) => {
                if (e.target === b) closeAllModals();
            });
        });

        // Settings interactive bindings
        document.getElementById('setting-theme').addEventListener('change', (e) => {
            applySettings({ theme: e.target.value });
        });
        document.getElementById('setting-font-family').addEventListener('change', (e) => {
            applySettings({ fontFamily: e.target.value });
        });
        document.getElementById('setting-font-size').addEventListener('input', (e) => {
            const val = parseInt(e.target.value, 10);
            const lbl = document.getElementById('setting-font-size-val');
            if (lbl) lbl.innerText = `${val}px`;
            applySettings({ fontSize: val });
        });
        document.getElementById('setting-line-height').addEventListener('change', (e) => {
            applySettings({ lineHeight: parseFloat(e.target.value) });
        });
        document.getElementById('setting-text-align').addEventListener('change', (e) => {
            applySettings({ textAlign: e.target.value });
        });
        document.getElementById('setting-spread').addEventListener('change', (e) => {
            applySettings({ spread: e.target.value });
        });
        document.getElementById('setting-flow').addEventListener('change', (e) => {
            applySettings({ flow: e.target.value });
        });
        document.getElementById('setting-max-width').addEventListener('change', (e) => {
            applySettings({ maxWidth: e.target.value });
        });
        document.getElementById('setting-padding').addEventListener('change', (e) => {
            applySettings({ pagePadding: parseInt(e.target.value, 10) });
        });

        document.getElementById('btn-reset-settings').addEventListener('click', () => {
            applySettings(defaultSettings);
        });

        document.getElementById('btn-search-go').addEventListener('click', async () => {
            const query = document.getElementById('txt-search').value.trim();
            if (!query) return;
            const res = await fetch(`/api/book/search?q=${encodeURIComponent(query)}`);
            const results = await res.json();
            const container = document.getElementById('search-results');
            container.innerHTML = '';
            results.forEach(r => {
                const div = document.createElement('div');
                div.className = 'search-result';
                div.innerHTML = `<div>${r.snippet}</div><div class="cfi-tag">${r.cfi}</div>`;
                div.onclick = () => loadSection(r.spine_index);
                container.appendChild(div);
            });
        });

        window.addEventListener('keydown', (e) => {
            if (e.key === 'Escape') {
                closeAllModals();
                if (sidebar.classList.contains('open')) {
                    sidebar.classList.remove('open');
                }
                return;
            }

            const activeTag = document.activeElement ? document.activeElement.tagName.toLowerCase() : '';
            if (activeTag === 'input' || activeTag === 'textarea' || activeTag === 'select') {
                return;
            }

            if (document.querySelector('.modal-backdrop.open')) {
                return;
            }

            if (e.key === 'ArrowRight' || e.key === ' ') {
                e.preventDefault();
                goNext();
            }
            if (e.key === 'ArrowLeft') {
                e.preventDefault();
                goPrev();
            }
        });

        initReader();
    </script>
</body>
</html>
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reader_html_structure_and_options() {
        assert!(READER_HTML.starts_with("<!DOCTYPE html>"));
        assert!(READER_HTML.trim_end().ends_with("</html>"));

        // Header options
        assert!(READER_HTML.contains("id=\"btn-open-settings\""));
        assert!(READER_HTML.contains("id=\"btn-open-about\""));

        // Sidebar options
        assert!(READER_HTML.contains("id=\"btn-sidebar-settings\""));
        assert!(READER_HTML.contains("id=\"btn-sidebar-about\""));

        // Settings modal & controls
        assert!(READER_HTML.contains("id=\"modal-settings\""));
        assert!(READER_HTML.contains("id=\"setting-theme\""));
        assert!(READER_HTML.contains("id=\"setting-font-family\""));
        assert!(READER_HTML.contains("id=\"setting-font-size\""));
        assert!(READER_HTML.contains("id=\"setting-line-height\""));
        assert!(READER_HTML.contains("id=\"setting-text-align\""));
        assert!(READER_HTML.contains("id=\"setting-spread\""));
        assert!(READER_HTML.contains("id=\"setting-flow\""));
        assert!(READER_HTML.contains("id=\"setting-max-width\""));
        assert!(READER_HTML.contains("id=\"setting-padding\""));
        assert!(READER_HTML.contains("id=\"btn-reset-settings\""));
        assert!(READER_HTML.contains("id=\"btn-close-settings\""));

        // About modal & metadata
        assert!(READER_HTML.contains("id=\"modal-about\""));
        assert!(READER_HTML.contains("id=\"about-version\""));
        assert!(READER_HTML.contains("id=\"about-description\""));
        assert!(READER_HTML.contains("id=\"about-book-title\""));
        assert!(READER_HTML.contains("id=\"about-book-author\""));
        assert!(READER_HTML.contains("id=\"about-book-id\""));
        assert!(READER_HTML.contains("id=\"about-book-spine\""));
        assert!(READER_HTML.contains("id=\"btn-close-about\""));

        // API Endpoint binding
        assert!(READER_HTML.contains("/api/app/info"));

        // Themes
        assert!(READER_HTML.contains("theme-dark"));
        assert!(READER_HTML.contains("theme-light"));
        assert!(READER_HTML.contains("theme-sepia"));
        assert!(READER_HTML.contains("theme-midnight"));
    }
}
