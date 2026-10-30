--!nonstrict

local helper = noctalia.expandPath("~/.local/bin/noctalia-obsidian")
if helper == "~/.local/bin/noctalia-obsidian" or helper == "" then
  local home = noctalia.getenv("HOME") or "/home/eno"
  helper = home .. "/.local/bin/noctalia-obsidian"
end
local notes_cache = {}
local query_seq = 0

local function shellQuote(v)
  return "'" .. tostring(v):gsub("'", "'\\''") .. "'"
end

local function parse_lines(stdout)
  local list = {}
  for line in (stdout or ""):gmatch("[^\r\n]+") do
    local ok, item = pcall(function()
      return noctalia.json.decode(line)
    end)
    if ok and type(item) == "table" and item.path then
      table.insert(list, item)
    end
  end
  return list
end

local function refresh_cache(clean, callback)
  query_seq += 1
  local seq = query_seq
  local cmd = helper .. " obs " .. shellQuote(clean)
  noctalia.runAsync(cmd, function(result)
    if seq ~= query_seq then return end
    if result and result.exitCode == 0 and result.stdout then
      local parsed = parse_lines(result.stdout)
      if callback then
        callback(parsed)
      end
    end
  end)
end

-- Initial load on startup
refresh_cache("", function(items)
  notes_cache = items
end)

local function filter_cache(clean)
  if clean == "" then
    return notes_cache
  end
  local q = string.lower(clean)
  local matches = {}
  for _, n in ipairs(notes_cache) do
    local t = string.lower(n.title or "")
    local d = string.lower(n.dir or "")
    if t:find(q, 1, true) or d:find(q, 1, true) then
      table.insert(matches, n)
    end
  end
  return matches
end

local function make_results(items)
  local rows = {}
  for i, n in ipairs(items) do
    if i > 50 then break end
    table.insert(rows, {
      id = "open:" .. n.path,
      title = n.title or "Untitled",
      subtitle = (n.dir and n.dir ~= "") and n.dir or "Vault root",
      glyph = "file-text",
    })
  end
  return rows
end

function onQuery(text)
  local query = text or ""
  local clean = query:match("^%s*(.-)%s*$") or ""

  -- 1. Synchronous results from cache
  local matched = filter_cache(clean)
  launcher.setResults(query, make_results(matched))

  -- 2. Asynchronous search update from helper
  refresh_cache(clean, function(items)
    if clean == "" then
      notes_cache = items
    end
    launcher.setResults(query, make_results(items))
  end)
end

function onActivate(id)
  if id and id:sub(1, 5) == "open:" then
    local path = id:sub(6)
    noctalia.runAsync(helper .. " open " .. shellQuote(path))
  end
end
