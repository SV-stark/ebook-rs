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
                <div id="annotations-list">No annotations yet. Select text in book to highlight.</div>
            </div>
        </sidebar>

        <div class="reader-stage" id="reader-stage">
            <button class="page-btn left" id="btn-prev">❮</button>
            <iframe id="section-frame" src="about:blank"></iframe>
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

        const sidebar = document.getElementById('sidebar');
        const sectionFrame = document.getElementById('section-frame');
        const selectionToolbar = document.getElementById('selection-toolbar');

        document.getElementById('btn-sidebar').addEventListener('click', () => {
            sidebar.classList.toggle('open');
        });

        function switchTab(tabId) {
            document.querySelectorAll('.tab-btn').forEach(b => b.classList.remove('active'));
            document.querySelectorAll('.tab-content').forEach(c => c.classList.remove('active'));
            event.target.classList.add('active');
            document.getElementById(tabId).classList.add('active');
            if (tabId === 'tab-annotations') refreshAnnotations();
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
                    if (item.href) loadSectionByHref(item.href);
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

        const sectionCache = new Map();

        async function getSectionHtml(index) {
            if (sectionCache.has(index)) {
                return sectionCache.get(index);
            }
            try {
                const res = await fetch(`/api/book/section/${index}`);
                if (!res.ok) return '';
                const html = await res.text();
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

            const isLight = document.body.classList.contains('theme-light');
            const isSepia = document.body.classList.contains('theme-sepia');
            const fgColor = isLight ? '#0f172a' : (isSepia ? '#451a03' : '#f8fafc');
            const pageBg = isLight ? '#ffffff' : (isSepia ? '#fbf0d9' : '#1e293b');
            const borderCol = isLight ? '#e2e8f0' : (isSepia ? '#e7d8b8' : '#334155');

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
                                font-family: Georgia, 'Times New Roman', serif;
                            }
                            .book-spread {
                                display: flex;
                                flex-direction: row;
                                gap: 28px;
                                height: calc(100vh - 40px);
                                max-width: 1350px;
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
                                padding: 28px 36px;
                                box-sizing: border-box;
                                border: 1px solid ${borderCol};
                                overflow: hidden;
                            }
                            .page-content {
                                flex: 1;
                                overflow-y: auto;
                                font-size: ${fontSize}px;
                                line-height: 1.7;
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
                            img { max-width: 100%; height: auto; }
                            mark { border-radius: 3px; padding: 2px 4px; }
                        </style>
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
                                font-family: Georgia, 'Times New Roman', serif;
                                font-size: ${fontSize}px;
                                line-height: 1.7;
                                color: ${fgColor};
                                padding: 32px 48px;
                                margin: 0 auto;
                                max-width: 800px;
                                box-sizing: border-box;
                            }
                            a { color: #3b82f6; }
                            img { max-width: 100%; height: auto; }
                            mark { border-radius: 3px; padding: 2px 4px; }
                        </style>
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

        // Feature 2: Live DOM Selection to CFI Bridge
        function setupIframeListeners() {
            const doc = sectionFrame.contentDocument || sectionFrame.contentWindow.document;
            doc.addEventListener('mouseup', () => {
                const sel = sectionFrame.contentWindow.getSelection();
                if (sel && sel.toString().trim().length > 0) {
                    selectedText = sel.toString().trim();
                    const range = sel.getRangeAt(0);
                    const spineStep = (currentSpineIndex + 1) * 2;
                    selectedCfiRange = `epubcfi(/6/${spineStep}!/4/2/1:${range.startOffset},/4/2/1:${range.endOffset})`;

                    // Show selection toolbar
                    const rect = range.getBoundingClientRect();
                    selectionToolbar.style.display = 'flex';
                    selectionToolbar.style.top = `${rect.top - 40}px`;
                    selectionToolbar.style.left = `${rect.left + (rect.width / 2) - 60}px`;
                } else {
                    selectionToolbar.style.display = 'none';
                }
            });
        }

        async function addHighlight(color) {
            if (!selectedCfiRange) return;
            await fetch('/api/annotations', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({
                    id: `ann-${Date.now()}`,
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
                    id: `bm-${Date.now()}`,
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
            isDoubleSpread = false;
            document.getElementById('btn-spread-single').classList.add('active');
            document.getElementById('btn-spread-double').classList.remove('active');
            loadSection(currentSpineIndex);
        });
        document.getElementById('btn-spread-double').addEventListener('click', () => {
            isDoubleSpread = true;
            document.getElementById('btn-spread-double').classList.add('active');
            document.getElementById('btn-spread-single').classList.remove('active');
            loadSection(currentSpineIndex);
        });

        // Feature 6: Flow Mode Handlers (Paginated vs Scrolled)
        document.getElementById('btn-flow-paginated').addEventListener('click', () => {
            isScrolledFlow = false;
            document.getElementById('reader-stage').classList.remove('scrolled-mode');
            document.getElementById('btn-flow-paginated').classList.add('active');
            document.getElementById('btn-flow-scrolled').classList.remove('active');
            loadSection(currentSpineIndex);
        });
        document.getElementById('btn-flow-scrolled').addEventListener('click', () => {
            isScrolledFlow = true;
            document.getElementById('reader-stage').classList.add('scrolled-mode');
            document.getElementById('btn-flow-scrolled').classList.add('active');
            document.getElementById('btn-flow-paginated').classList.remove('active');
            loadSection(currentSpineIndex);
        });

        document.getElementById('btn-prev').addEventListener('click', goPrev);
        document.getElementById('btn-next').addEventListener('click', goNext);

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
                div.innerHTML = `<div>${r.snippet}</div><div class="cfi-tag">${r.cfi}</div>`;
                div.onclick = () => loadSection(r.spine_index);
                container.appendChild(div);
            });
        });

        window.addEventListener('keydown', (e) => {
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
