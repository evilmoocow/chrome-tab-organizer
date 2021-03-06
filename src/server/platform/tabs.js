goog.provide("platform.tabs")
goog.provide("platform.windows")

goog.require("util.cell")
goog.require("util.math")
goog.require("util.log")
goog.require("util.array")

goog.scope(function () {
  var cell   = util.cell
    , array  = util.array
    , log    = util.log.log
    , assert = util.log.assert
    , math   = util.math

  /**
   * @type {!Array.<!Win>}
   */
  var aWins = []
  var cWins = {}
  var cTabs = {}

  var windows = chrome["windows"]
    , tabs    = chrome["tabs"]

  /**
   * @constructor
   */
  function Win(x) {
    /**
     * @type {!Array.<!Tab>}
     */
    this.tabs  = []
    this.id    = x["id"]
    this.state = x["state"] // TODO is this necessary/useful...?
    this.index = array.push(aWins, this)

    var self = this
    if (x["tabs"] != null) {
      array.each(x["tabs"], function (t) {
        array.push(self.tabs, new Tab(t, self))
      })
    }

    cWins[this.id] = this
  }

  // TODO handle lastFocusedTab ?
  function transfer(tab, t) {
    tab.id      = t["id"]
    tab.focused = t["active"]
    tab.index   = t["index"]
    tab.pinned  = t["pinned"]
    tab.url     = t["url"]   || ""
    tab.title   = t["title"] || tab.url
  }

  /**
   * @constructor
   */
  function Tab(x, win) {
    assert(win != null)

    this.window = win
    transfer(this, x)

    // TODO what if a tab is already focused ?
    if (this.focused) {
      this.window.lastFocusedTab = this
    }

    cTabs[this.id] = this
  }

  platform.tabs.on             = {}
  platform.tabs.on.created     = cell.value(undefined)
  platform.tabs.on.updated     = cell.value(undefined)
  platform.tabs.on.removed     = cell.value(undefined)
  platform.tabs.on.focused     = cell.value(undefined)
  platform.tabs.on.unfocused   = cell.value(undefined)
  platform.tabs.on.moved       = cell.value(undefined)
  platform.tabs.on.updateIndex = cell.value(undefined)

  platform.windows.on          = {}
  platform.windows.on.created  = cell.value(undefined)
  platform.windows.on.removed  = cell.value(undefined)

  platform.tabs.loaded = platform.windows.loaded = cell.dedupe(false)

  /**
   * @return {!Array.<!Win>}
   */
  platform.windows.getAll = function () {
    assert(platform.windows.loaded.get(), "platform.windows.loaded")
    return aWins
  }

  /**
   * @return {!Array.<!Tab>}
   */
  platform.tabs.getAll = function () {
    assert(platform.tabs.loaded.get(), "platform.tabs.loaded")
    var r = []
    array.each(aWins, function (x) {
      array.each(x.tabs, function (x) {
        array.push(r, x)
      })
    })
    return r
  }

  // TODO create two new tabs, close them, refresh the popup

  platform.windows.get = function (i) {
    assert(i in cWins)
    return cWins[i]
  }

  platform.tabs.get = function (i) {
    assert(i in cTabs)
    return cTabs[i]
  }

  /**
   * @param {number} id
   * @param {!Object.<string,number>} o
   * @param {function():void=} f
   */
  function moveWindow(id, o, f) {
    windows["update"](id, {
      "top":    o.top,
      "left":   o.left,
      "width":  o.width,
      "height": o.height,
      "state":  "normal"
    }, function () {
      if (f != null) {
        f()
      }
    })
  }

  platform.windows.move = function (id, o) {
    moveWindow(id, o, function () {
      setTimeout(function () {
        moveWindow(id, o)
      }, 100)
    })
  }

  platform.windows.maximize = function (id) {
    windows["update"](id, { "state": "maximized" })
  }

  /**
   * @param {!Array.<number>} a
   */
  platform.tabs.close = function (a) {
    tabs["remove"](a)
  }

  // TODO update an existing New Tab page, if it exists ?
  /**
   * @param {string} url
   * @param {boolean} pinned
   */
  platform.tabs.open = function (url, pinned) {
    tabs["create"]({
      "url":    url,
      "active": true,
      "pinned": !!pinned
    }, function (o) {
      log("1", o)
    })
  }

  /**
   * @param {!Array.<number>} a
   * @param {number} index
   * @param {number} win
   */
  platform.tabs.move = function (a, index, win) {
    assert(win in cWins)
    array.each(a, function (x, i) {
      var tab = platform.tabs.get(x)
      log(tab.window.id, win)
      // TODO is this correct ?
      //if (x.index !== index) {
      //log(x.title, index, index)
      tabs["move"](tab.id, {
        "index": (tab.window.id === win && tab.index < index
                   ? index - 1
                   : index + i),
        "windowId": win
      })
      //}
    })
  }

  /**
   * @param {number} i
   */
  platform.tabs.focus = function (i) {
    var tab = platform.tabs.get(i)
    tabs["update"](tab.id, { "active": true })
    assert(tab.window != null)
    windows["update"](tab.window.id, { "focused": true })
  }

  function updateIndices(a, iMin) {
    var r = []
    for (var i = iMin, iLen = array.len(a); i < iLen; ++i) {
      var x = a[i]
      if (x.index !== i) {
        x.index = i
        array.push(r, x)
      }
    }
    return r
  }

  function updateWindowIndices(a, iMin) {
    /*var r = */updateIndices(a, iMin)
    /*if (array.len(r)) {
      platform.windows.on.updateIndex.set(r)
    }*/
  }

  function updateTabIndices(a, iMin) {
    var r = updateIndices(a, iMin)
    if (array.len(r)) {
      platform.tabs.on.updateIndex.set(r)
    }
  }

  function updateTab(tab, t) {
    assert(tab.index === t["index"])

    delete cTabs[tab.id]
    transfer(tab, t)
    cTabs[tab.id] = tab

    assert(tab.window != null)
    assert(tab.window.id === t["windowId"])

    platform.tabs.on.updated.set(tab)
  }

  function focus1(tab, win) {
    win.lastFocusedTab = tab
    tab.focused = true
    platform.tabs.on.focused.set(tab)
  }

  function focus(tab) {
    var win = tab.window
    if (win != null) {
      var old = win.lastFocusedTab
      if (old == null) {
        focus1(tab, win)
      } else if (old !== tab) {
        old.focused = false
        platform.tabs.on.unfocused.set(old)
        focus1(tab, win)
      }
    }
  }

  function onCreated(t) {
    log("2", t)
    var old = cTabs[t["id"]]
    if (old == null) {
      var win = cWins[t["windowId"]]
      if (win != null) {
        var tab = new Tab(t, win)

        array.insertAt(win.tabs, tab.index, tab)
        updateTabIndices(win.tabs, tab.index + 1)

        platform.tabs.on.created.set(tab)
      }
    } else {
      updateTab(old, t)
    }
  }

  addEventListener("load", function () {
    windows["getAll"]({ "populate": true }, function (a) {
      array.each(a, function (w) {
        if (w["type"] === "normal") {
          new Win(w)
        }
      })

      windows["onCreated"]["addListener"](function (w) {
        if (w["type"] === "normal") {
          var win = new Win(w)
          platform.windows.on.created.set(win)
        }
      })

      windows["onRemoved"]["addListener"](function (id) {
        var win = cWins[id]
        if (win != null) {
          delete cWins[id]

          assert(typeof win.index === "number")
          assert(win.index >= 0)
          assert(win.index < array.len(aWins))

          array.removeAt(aWins, win.index)
          updateWindowIndices(aWins, win.index)

          platform.windows.on.removed.set(win)
        }
      })

      tabs["onCreated"]["addListener"](onCreated)
      tabs["onUpdated"]["addListener"](function (id, info, t) {
        onCreated(t)
      })

      tabs["onRemoved"]["addListener"](function (id, info) {
        var tab = cTabs[id]
        if (tab != null) {
          assert(id === tab.id)
          delete cTabs[id]

          var win = tab.window
          assert(win != null)
          array.removeAt(win.tabs, tab.index)
          updateTabIndices(win.tabs, tab.index)

          platform.tabs.on.removed.set({
            windowClosing: info["isWindowClosing"],
            tab: tab
          })
        }
      })

      /*;(function () {
        var a = null

        function push(f) {
          if (a === null) {
            a = []
            setTimeout(function () {
              array.each(a, function (f) {
                f()
              })
              a = null
            }, 100)
          }
          array.push(a, f)
        }*/

        tabs["onMoved"]["addListener"](function (id, info) {
          var tab = cTabs[id]
          if (tab != null) {
            //tabs["get"](id, function (t) {
              assert(tab.index === info["fromIndex"])

              var win = tab.window
              assert(win != null)

              var oldIndex = tab.index

              array.removeAt(win.tabs, oldIndex)
              array.insertAt(win.tabs, info["toIndex"], tab)
              updateTabIndices(win.tabs, math.min(oldIndex, info["toIndex"] + 1))

              // TODO is this reliable ?
              //tab.index = t["index"]
              tab.index = info["toIndex"]

              //log(tab.title, info["toIndex"], t["index"])

              assert(oldIndex !== tab.index)

              platform.tabs.on.moved.set(tab)
            //})
          }
        })

        // TODO what about detaching a focused tab ?
        tabs["onDetached"]["addListener"](function (id, info) {
          var tab = cTabs[id]
          if (tab != null) {
            var win = tab.window
            assert(win != null)

            // TODO remove all the checks that see if tab.window is null or not ?
            //delete tab.window

            assert(win.id === info["oldWindowId"])
            assert(tab.index === info["oldPosition"])

            array.removeAt(win.tabs, tab.index)
            updateTabIndices(win.tabs, tab.index)
          }
        })

        // TODO what about attaching a focused tab ?
        tabs["onAttached"]["addListener"](function (id, info) {
          var tab = cTabs[id]
          if (tab != null) {
            //tabs["get"](id, function (t) {
              var win = cWins[info["newWindowId"]]
              assert(win != null)

              assert(win.id === info["newWindowId"])

              tab.window = win

              array.insertAt(win.tabs, info["newPosition"], tab)
              updateTabIndices(win.tabs, info["newPosition"] + 1)

              tab.index = info["newPosition"]

              //log(tab.title, info["newPosition"], t["index"])

              platform.tabs.on.moved.set(tab)
            //})
          }
        })
      //})()

      tabs["onActivated"]["addListener"](function (info) {
        var tab = cTabs[info["tabId"]]
        if (tab != null) {
          assert(tab.window != null)
          assert(tab.window.id === info["windowId"])
          focus(tab)
        }
      })

      tabs["onReplaced"]["addListener"](function (addedId, removedId) {
        tabs["get"](addedId, function (tab) {
          var old = cTabs[removedId]
          if (old != null) {
            assert(old.id !== tab["id"])
            assert(old.id === removedId)
            assert(tab["id"] === addedId)
            updateTab(old, tab)
          }
        })
      })

      platform.tabs.loaded.set(true)
      platform.windows.loaded.set(true)
    })
  }, true)
})
