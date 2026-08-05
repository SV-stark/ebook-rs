/// Embedded modern HTML/CSS/JS reader web application interface matching EPUB.js functionality.
pub const READER_HTML: &str = r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>EBook-RS Reader</title>
    <link rel="preconnect" href="https://fonts.googleapis.com">
    <link rel="preconnect" href="https://fonts.gstatic.com" crossorigin>
    <link href="https://fonts.googleapis.com/css2?family=Inter:wght@300;400;500;600;700&family=Merriweather:ital,wght@0,300;0,400;0,700;1,300&family=Fira+Code&display=swap" rel="stylesheet">
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

        * { box-sizing: border-box; margin: 0; padding: 0; }

        body {
            font-family: 'Inter', sans-serif;
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
            gap: 12px;
        }

        .book-title {
            font-weight: 600;
            font-size: 0.95rem;
            max-width: 400px;
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

        .btn-active {
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

        /* TOC Tree */
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

        /* Reader Stage / Iframe Frame */
        .reader-stage {
            flex: 1;
            display: flex;
            justify-content: center;
            align-items: center;
            position: relative;
            background: var(--bg-primary);
            overflow: hidden;
        }

        iframe#section-frame {
            width: 100%;
            height: 100%;
            border: none;
            background: transparent;
        }

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

        /* Bottom Status & Progress Footer */
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

        /* Search input inside sidebar */
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

        .search-result {
            padding: 10px;
            background: var(--bg-hover);
            border-radius: 6px;
            margin-bottom: 8px;
            cursor: pointer;
            font-size: 0.85rem;
        }

        .search-result:hover {
            border: 1px solid var(--accent);
        }

        .search-cfi {
            font-size: 0.75rem;
            color: var(--accent);
            margin-top: 4px;
            font-family: 'Fira Code', monospace;
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
            <button class="btn" id="btn-theme-dark">🌙 Dark</button>
            <button class="btn" id="btn-theme-light">☀️ Light</button>
            <button class="btn" id="btn-theme-sepia">📜 Sepia</button>
            <button class="btn" id="btn-font-dec">A-</button>
            <button class="btn" id="btn-font-inc">A+</button>
        </div>
    </header>

    <div class="main-container">
        <sidebar id="sidebar">
            <div class="sidebar-tabs">
                <button class="tab-btn active" onclick="switchTab('tab-toc')">Contents</button>
                <button class="tab-btn" onclick="switchTab('tab-search')">Search</button>
                <button class="tab-btn" onclick="switchTab('tab-annotations')">Annotations</button>
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
                <div id="annotations-list">No annotations yet. Select text in book to add highlight.</div>
            </div>
        </sidebar>

        <div class="reader-stage">
            <button class="page-btn left" id="btn-prev">❮</button>
            <iframe id="section-frame" src="about:blank"></iframe>
            <button class="page-btn right" id="btn-next">❯</button>
        </div>
    </div>

    <footer>
        <div id="lbl-location">Spine 1 / 1</div>
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
        let bookData = null;

        const sidebar = document.getElementById('sidebar');
        const sectionFrame = document.getElementById('section-frame');

        document.getElementById('btn-sidebar').addEventListener('click', () => {
            sidebar.classList.toggle('open');
        });

        function switchTab(tabId) {
            document.querySelectorAll('.tab-btn').forEach(b => b.classList.remove('active'));
            document.querySelectorAll('.tab-content').forEach(c => c.classList.remove('active'));
            event.target.classList.add('active');
            document.getElementById(tabId).classList.add('active');
        }

        async function initReader() {
            try {
                const metaRes = await fetch('/api/book/metadata');
                const meta = await metaRes.json();
                document.getElementById('lbl-title').innerText = meta.title || "EPUB Book";

                const spineRes = await fetch('/api/book/spine');
                const spine = await spineRes.json();
                totalSpineSections = spine.length;

                const tocRes = await fetch('/api/book/toc');
                const toc = await tocRes.json();
                renderToc(toc);

                loadSection(0);
            } catch (err) {
                console.error("Initialization failed:", err);
            }
        }

        function renderToc(items, container = document.getElementById('toc-list')) {
            container.innerHTML = '';
            items.forEach(item => {
                const div = document.createElement('div');
                div.className = 'toc-item';
                div.innerText = item.label;
                div.onclick = () => {
                    if (item.href) {
                        loadSectionByHref(item.href);
                    }
                };
                container.appendChild(div);
                if (item.subitems && item.subitems.length > 0) {
                    const subContainer = document.createElement('div');
                    subContainer.className = 'toc-sub';
                    renderToc(item.subitems, subContainer);
                    container.appendChild(subContainer);
                }
            });
        }

        async function loadSection(index) {
            if (index < 0 || index >= totalSpineSections) return;
            currentSpineIndex = index;
            const res = await fetch(`/api/book/section/${index}`);
            const html = await res.text();
            
            sectionFrame.srcdoc = `
                <html>
                <head>
                    <style>
                        body {
                            font-family: 'Merriweather', serif;
                            font-size: ${fontSize}px;
                            line-height: 1.7;
                            color: ${document.body.classList.contains('theme-light') ? '#0f172a' : '#f8fafc'};
                            padding: 32px 48px;
                            margin: 0 auto;
                            max-width: 800px;
                        }
                        a { color: #3b82f6; }
                        img { max-width: 100%; height: auto; }
                    </style>
                </head>
                <body>${html}</body>
                </html>
            `;

            updateFooter();
        }

        async function loadSectionByHref(href) {
            const cleanHref = href.split('#')[0];
            const spineRes = await fetch('/api/book/spine');
            const spine = await spineRes.json();
            const idx = spine.findIndex(s => s.href.includes(cleanHref) || s.idref === cleanHref);
            if (idx !== -1) {
                loadSection(idx);
            } else {
                loadSection(0);
            }
        }

        function updateFooter() {
            document.getElementById('lbl-location').innerText = `Section ${currentSpineIndex + 1} of ${totalSpineSections}`;
            const pct = Math.round(((currentSpineIndex + 1) / totalSpineSections) * 100);
            document.getElementById('lbl-progress-val').innerText = `${pct}%`;
            document.getElementById('slider-progress').value = pct;
            const spineStep = (currentSpineIndex + 1) * 2;
            document.getElementById('lbl-cfi').innerText = `epubcfi(/6/${spineStep}!/4/2/1:0)`;
        }

        document.getElementById('btn-prev').addEventListener('click', () => loadSection(currentSpineIndex - 1));
        document.getElementById('btn-next').addEventListener('click', () => loadSection(currentSpineIndex + 1));

        document.getElementById('btn-theme-dark').addEventListener('click', () => {
            document.body.className = 'theme-dark';
            loadSection(currentSpineIndex);
        });
        document.getElementById('btn-theme-light').addEventListener('click', () => {
            document.body.className = 'theme-light';
            loadSection(currentSpineIndex);
        });
        document.getElementById('btn-theme-sepia').addEventListener('click', () => {
            document.body.className = 'theme-sepia';
            loadSection(currentSpineIndex);
        });

        document.getElementById('btn-font-inc').addEventListener('click', () => {
            fontSize += 2;
            loadSection(currentSpineIndex);
        });
        document.getElementById('btn-font-dec').addEventListener('click', () => {
            if (fontSize > 12) fontSize -= 2;
            loadSection(currentSpineIndex);
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
                div.innerHTML = `<div>${r.snippet}</div><div class="search-cfi">${r.cfi}</div>`;
                div.onclick = () => loadSection(r.spine_index);
                container.appendChild(div);
            });
        });

        // Keybindings
        window.addEventListener('keydown', (e) => {
            if (e.key === 'ArrowRight' || e.key === ' ') loadSection(currentSpineIndex + 1);
            if (e.key === 'ArrowLeft') loadSection(currentSpineIndex - 1);
        });

        initReader();
    </script>
</body>
</html>
"#;
