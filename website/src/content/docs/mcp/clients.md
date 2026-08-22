---
title: AI Client Setup (Claude, Cursor, Antigravity)
description: Configure ebook-rs MCP server in popular AI assistants.
---

import { Tabs, TabItem } from '@astrojs/starlight/components';

<Tabs>
  <TabItem label="Claude Desktop">
Add to your `claude_desktop_config.json`:

```json
{
  "mcpServers": {
    "ebook-rs": {
      "command": "ebook-rs",
      "args": ["mcp"]
    }
  }
}
```
  </TabItem>

  <TabItem label="Cursor">
In Cursor Settings > Features > MCP Servers:
- **Name**: `ebook-rs`
- **Type**: `command`
- **Command**: `ebook-rs mcp`
  </TabItem>

  <TabItem label="Antigravity / VS Code">
Add to your settings or sidecar configuration:

```json
{
  "mcpServers": {
    "ebook-rs": {
      "command": "ebook-rs",
      "args": ["mcp"]
    }
  }
}
```
  </TabItem>
</Tabs>\n