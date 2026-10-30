--!nonstrict

local helper = noctalia.expandPath("~/.local/bin/noctalia-obsidian")
if helper == "~/.local/bin/noctalia-obsidian" or helper == "" then
  local home = noctalia.getenv("HOME") or "/home/eno"
  helper = home .. "/.local/bin/noctalia-obsidian"
end

local function shellQuote(v)
  return "'" .. tostring(v):gsub("'", "'\\''") .. "'"
end

function onQuery(text)
  local query = text or ""
  local clean = query:match("^%s*(.-)%s*$") or ""

  local results = {}
  if clean == "" then
    table.insert(results, {
      id = "open",
      title = "Open today's daily note",
      subtitle = "Creates from template if missing",
      glyph = "calendar",
    })
  else
    table.insert(results, {
      id = "add:" .. clean,
      title = "Add to daily note: " .. clean,
      subtitle = "Append entry under today's heading",
      glyph = "pencil-plus",
    })
  end

  launcher.setResults(query, results)
end

function onActivate(id)
  if id == "open" then
    noctalia.runAsync(helper .. " day")
  elseif id and id:sub(1, 4) == "add:" then
    local entry = id:sub(5)
    noctalia.runAsync(helper .. " day " .. shellQuote(entry))
  end
end
