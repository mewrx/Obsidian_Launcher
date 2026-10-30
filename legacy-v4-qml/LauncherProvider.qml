import QtQuick
import Quickshell
import Quickshell.Io
import qs.Commons

Item {
  id: root

  property var pluginApi: null
  property var launcher: null
  property string name: "Obsidian"
  property bool handleSearch: true          // lets us see plain text like "/day foo"
  property string supportedLayouts: "list"

  readonly property string helper: Quickshell.env("HOME") + "/.local/bin/noctalia-obsidian"

  // /obs cache (getResults must be synchronous, helper runs async)
  property string obsQuery: ""
  property bool obsLoaded: false
  property var obsCache: []
  property var pendingQuery: null

  // "/day", "/obs" (plain text) or ">note", ">obs" (command mode). ">day" is taken by the weather plugin.
  function parse(t) {
    var m = t.match(/^(?:\/(day|obs)|>(note|obs))(?:\s+([\s\S]*))?$/)
    if (!m) return null
    var kind = m[1] || m[2]
    return { kind: (kind === "note" ? "day" : kind), arg: (m[3] || "").trim() }
  }

  function handleCommand(searchText) { return parse(searchText) !== null }

  function commands() {
    return [
      { "name": ">note", "description": "Add to / open today's daily note", "icon": "calendar", "isTablerIcon": true,
        "onActivate": function() { launcher.setSearchText(">note ") } },
      { "name": ">obs", "description": "Search Obsidian notes", "icon": "notebook", "isTablerIcon": true,
        "onActivate": function() { launcher.setSearchText(">obs ") } }
    ]
  }

  function onOpened() { obsLoaded = false }

  function getResults(searchText) {
    var p = parse(searchText)
    if (!p) return []

    if (p.kind === "day") {
      if (p.arg === "") {
        return [{
          "name": "Open today's daily note", "description": "Creates it from the template if missing",
          "icon": "calendar", "isTablerIcon": true,
          "onActivate": function() { Quickshell.execDetached([root.helper, "day"]); launcher.close() }
        }]
      }
      var text = p.arg
      return [{
        "name": "Add to daily note", "description": text,
        "icon": "pencil-plus", "isTablerIcon": true,
        "onActivate": function() { Quickshell.execDetached([root.helper, "day", text]); launcher.close() }
      }]
    }

    // obs
    if (!obsLoaded || p.arg !== obsQuery) requestObs(p.arg)
    return obsCache.map(function(n) {
      return {
        "name": n.title, "description": n.dir, "icon": "file-text", "isTablerIcon": true, "singleLine": true,
        "onActivate": function() { Quickshell.execDetached([root.helper, "open", n.path]); launcher.close() }
      }
    })
  }

  function requestObs(q) {
    if (obsProc.running) { pendingQuery = q; return }
    obsQuery = q
    obsProc.command = [helper, "obs", q]
    obsProc.running = true
  }

  Process {
    id: obsProc
    stdout: StdioCollector {
      onStreamFinished: {
        var items = []
        var lines = text.split("\n")
        for (var i = 0; i < lines.length; i++) {
          if (!lines[i]) continue
          try { items.push(JSON.parse(lines[i])) } catch (e) {}
        }
        root.obsCache = items
        root.obsLoaded = true
        var pq = root.pendingQuery
        root.pendingQuery = null
        if (pq !== null && pq !== root.obsQuery) root.requestObs(pq)
        else if (root.launcher) root.launcher.updateResults()
      }
    }
  }
}
