goog.provide("logic")

goog.require("platform.manifest")
goog.require("util.Symbol")
goog.require("util.cell")
goog.require("util.array")
goog.require("util.object")
goog.require("util.url")
goog.require("util.math")
goog.require("util.dom")
goog.require("util.log")
goog.require("util.string")
goog.require("util.time")
goog.require("menus.tab")
goog.require("ui.menu")
goog.require("ui.group")
goog.require("ui.tab")
goog.require("ui.animate")
goog.require("ui.layout")
goog.require("tabs")
goog.require("opt")
goog.require("search")

goog.scope(function () {
  var cell     = util.cell
    , array    = util.array
    , object   = util.object
    , math     = util.math
    , url      = util.url
    , log      = util.log.log
    , assert   = util.log.assert
    , fail     = util.log.fail
    , Symbol   = util.Symbol
    , manifest = platform.manifest

  var info  = Symbol("info")
    , group = Symbol("group")

  logic.info = info

  util.dom.title(manifest.get("name"))

  var hiddenGroupList = ui.animate.object({
    //transformOrigin: "100% 50%",
    //marginTop: "5px",
    //marginLeft: "5px",
    //paddingBottom: "50px",
    //marginBottom: "50px",
    //rotationX: 0.5,
    //rotationY: 20,
    //rotationZ: -0.25,
    //height: "90%",
    //scaleY: "0.9"

    //scale: 0.98,
    "opacity": "0"
  })

  //var hiddenGroupList2 = Object.create(hiddenGroupList)
  //hiddenGroupList2.clearProps = "scale,opacity"

  // TODO util.string ?
  function pluralize(x, s) {
    if (x === 1) {
      return x + s
    } else {
      return x + s + "s"
    }
  }

  var defaultTabSort = function (x, y) {
    x = x[info]
    y = y[info]
    if (x.type === "active" && y.type === "active") {
      return x.index <= y.index
    } else if (x.type === "active") {
      return true
    } else if (y.type === "active") {
      return false
    } else {
      // TODO is this right ?
      // TODO should sort by time added to the group ?
      return (x.time.unloaded || x.time.focused || x.time.updated || x.time.created) >=
             (y.time.unloaded || y.time.focused || y.time.updated || y.time.created)
    }
  }

  var makeGroupSort = (function () {
    function getDate(diff) {
      if (diff.day === 0) {
        if (diff.hour === 0) {
          return "Less than an hour ago"
        } else {
          return pluralize(diff.hour, " hour") + " ago"
        }
      } else {
        var hours = diff.hour - (diff.day * 24)
        return pluralize(diff.day, " day") + " " + pluralize(hours, " hour") + " ago"
      }

      /*var i1 = util.time.toLocalTime(t1)
        , i2 = util.time.toLocalTime(t2)
      i1 = math.floor(i1 / util.time.day)
      i2 = math.floor(i2 / util.time.day)
      var i = i2 - i1*/

      /*year = (year === 0
               ? ""
               : (year === 1
                   ? year + " year "
                   : year + " years "))

      month = (month === 0
                ? ""
                : (month === 1
                    ? month + " month "
                    : month + " months "))*/
    }

    return function (f) {
      return {
        groupSort: function (x, y) {
          return x.id > y.id
        },
        tabSort: defaultTabSort,
        /*function (x, y) {
          return f(x[info]) >= f(y[info])
        },*/
        init: function (tab) {
          var now = util.time.roundToHour(util.time.now())
          var id = util.time.roundToHour(f(tab))
          var diff = util.time.difference(id, now)
                   /*(diff.day === 0
                     ?
                     : util.time.roundToDay(then))*/
          return [{
            id: id,
            // TODO update every 5 minutes or whatever
            name: cell.dedupe(getDate(diff)),
            rename: false
          }]
        }
      }
    }
  })()

  function lookup(o) {
    return function (x) {
      assert(x in o)
      return o[x]
    }
  }

  // TODO generic utility
  function setNew(o, k, f) {
    if (o[k] == null) {
      o[k] = f()
    }
    return o[k]
  }

  logic.initialize = function (e) {
    var groupSort = e.bind([opt.get("group.sort.type")], lookup({
      "group": {
        groupSort: function (x, y) {
          if (x.type === "window" && y.type === "window") {
            return x.index <= y.index
          } else if (x.type === "window") {
            return true
          } else if (y.type === "window") {
            return false
          } else if (x.id === "") {
            return false
          } else if (y.id === "") {
            return true
          } else {
            // TODO code duplication with "name" sort
            return util.string.upperSorter(x.id, y.id) <= 0
          }
        },
        tabSort: defaultTabSort,
        init: function (tab) {
          var r = []
          if (tab.type !== "unloaded") {
            assert(tab.window != null)
            array.push(r, {
              type: "window",
              id: tab.window.id,
              name: tab.window.name,
              index: tab.window.time.created,
              rename: true
            })
          }
          object.each(tab.groups, function (_, s) {
            array.push(r, {
              type: "group",
              id: s,
              name: cell.dedupe(s),
              rename: true
            })
          })
          if (array.len(r) === 0) {
            array.push(r, {
              type: "group",
              id: "",
              name: cell.dedupe(""),
              rename: false // TODO allow for renaming this...?
            })
          }
          return r
        }
      },
      // TODO o.time.session
      "session": makeGroupSort(function (o) {
        return o.time.session || o.time.unloaded || o.time.created
      }),
      "created": makeGroupSort(function (o) {
        return o.time.created
      }),
      "focused": makeGroupSort(function (o) {
        //return o.time.updated || o.time.created
        return o.time.focused || o.time.created
      }),
      "name": {
        groupSort: function (x, y) {
          return util.string.upperSorter(x.id, y.id) <= 0
        },
        tabSort: function (x, y) {
          return util.string.upperSorter(x[info].title, y[info].title) <= 0
        },
        init: function (tab) {
          if (tab.title === "") {
            return [{
              id: "",
              name: cell.dedupe(""),
              rename: false
            }]
          } else {
            var s = util.string.upper(tab.title[0])
            return [{
              id: s,
              name: cell.dedupe(s),
              rename: false
            }]
          }
        }
      },
      "url": {
        groupSort: function (x, y) {
          if (x.id === "chrome://") {
            return true
          } else if (y.id === "chrome://") {
            return false
          } else {
            return util.string.upperSorter(x.id, y.id) <= 0
          }
        },
        tabSort: function (x, y) {
          return x[info].url <= y[info].url
          // TODO pretty inefficient
          // return url.printURI(url.simplify(x.info.location)) < url.printURI(url.simplify(y.info.location))
        },
        init: function (tab) {
          var s = url.simplify(tab.location)
          var name = (s.scheme === "chrome:"
                       ? "chrome://"
                       : s.scheme + s.separator + s.authority + s.hostname + s.port)
          return [{
            id: name,
            name: cell.dedupe(name),
            rename: false
          }]
        }
      }
    }))

    var oGroups = {}
      , aGroups = []

    function hide(e, i, f) {
      ui.animate.to(e, i, hiddenGroupList, function () {
        e.visible.set(false)
        f()
      })
    }

    function show(e, i) {
      e.visible.set(true)
      ui.animate.from(e, i, hiddenGroupList)
    }

    function reset() {
      array.each(aGroups, function (oGroup) {
        oGroup.element.remove()
      })
      oGroups = {}
      aGroups = []
    }

    function removeTabIf(tab, animate, f) {
      var toRemove = []
      array.each(aGroups, function (oGroup) {
        var o = oGroup.oTabs[tab.id]
        if (o != null && f(oGroup)) {
          delete oGroup.oTabs[tab.id]
          removeTabFrom(o, oGroup, animate)

          if (array.len(oGroup.aTabs) === 0) {
            log(oGroup.id)
            delete oGroups[oGroup.id]
            array.push(toRemove, oGroup)
            if (animate) {
              ui.group.hide(oGroup.element, 1)
            } else {
              oGroup.element.remove()
            }
          }
        }
      })
      // TODO can be slightly more efficient
      // TODO use array.removeSorted ?
      array.each(toRemove, function (oGroup) {
        array.remove(aGroups, oGroup)
      })
    }

    // TODO inefficient
    function searchTabs(f) {
      var iTabs   = 0
        , rGroups = []

      var seen = {}

      array.each(aGroups, function (oGroup) {
        var visible = false

        array.each(oGroup.aTabs, function (x) {
          x[info].visible = (f === false || f(x[info]))
          x.visible.set(x[info].visible)
          if (x[info].visible) {
            if (!seen[x[info].id]) {
              ++iTabs
            }
            visible = true
          }
          seen[x[info].id] = true
        })

        if (visible) {
          array.push(rGroups, oGroup.element)
        }
        oGroup.element.visible.set(visible)
      })

      var iGroups = array.len(rGroups)

      // TODO code duplication
      var sTabs = (iTabs === 1
                    ? iTabs + " tab"
                    : iTabs + " tabs")

      // TODO code duplication
      var sGroups = (iGroups === 1
                      ? iGroups + " group"
                      : iGroups + " groups")

      ui.layout.visibleGroups.set(rGroups)

      util.dom.title(manifest.get("name") + " - " + sTabs + " in " + sGroups)
    }

    function addGroups(e, tab, animate, f) {
      var sort = groupSort.get()
      array.each(sort.init(tab), function (o) {
        f(setNew(oGroups, o.id, function () {
          o.oTabs = {}
          o.aTabs = []
          o.element = ui.group.make(o.name, o, function (e) {
            o.tabList = e
          }, function (normal, selected) {
            array.each(o.aTabs, function (x) {
              x = x[info]
              if (x.visible) {
                if (x.selected) {
                  array.push(selected, x)
                } else {
                  array.push(normal, x)
                }
              }
            })
          })

          var a     = aGroups
          var index = array.insertSorted(a, o, sort.groupSort)
          var elem  = a[index + 1]
          if (elem == null) {
            o.element.move(e)
          } else {
            o.element.moveBefore(elem.element)
          }

          if (animate) {
            ui.group.show(o.element, 1)
          }
          return o
        }))
      })
    }

    // TODO this whole thing dealing with selection is hacky, try and refactor it
    function deselectAllTabs(oGroup) {
      delete oGroup.previouslySelected
      tabs.deselect(array.map(oGroup.aTabs, function (x) {
        return x[info]
      }))
    }

    function selectTab(oGroup, oTab) {
      // TODO needs to change when updating
      oGroup.previouslySelected = oTab.id
      if (!oTab.selected) {
        tabs.select([oTab])
      }
    }

    // TODO
    function tabClick(tab, click) {
      var oGroup = tab[group]
        , oTab   = tab[info]
      if (click.left) {
        if (click.ctrl) {
          if (oTab.selected) {
            // TODO is this correct ?
            delete oGroup.previouslySelected
            tabs.deselect([oTab])
          } else {
            selectTab(oGroup, oTab)
          }

        } else if (click.shift) {
          if (oGroup.previouslySelected) {
            var start = false
              , aYes  = []
              , aNo   = []
            array.each(oGroup.aTabs, function (x) {
              // TODO what if previouslySelected is oTab?
              var i = x[info].id
                , b = (i === oTab.id || i === oGroup.previouslySelected)

                       // TODO inefficient
              if (b && oTab.id !== oGroup.previouslySelected) {
                start = !start
              }

              if (b || start) {
                if (!x[info].selected) {
                  array.push(aYes, x[info])
                }
              } else {
                if (x[info].selected) {
                  array.push(aNo, x[info])
                }
              }
            })
            tabs.select(aYes)
            tabs.deselect(aNo)
          } else {
            selectTab(oGroup, oTab)
          }

        // TODO behavior for this ?
        } else if (click.alt) {

        } else {
          // TODO ew
          switch (opt.get("tabs.click.type").get()) {
          case "select-focus":
            if (oTab.selected) {
              tabs.focus(oTab)
            } else {
              deselectAllTabs(oGroup)
              selectTab(oGroup, oTab)
            }
            break
          case "focus":
            if (!oTab.selected) {
              deselectAllTabs(oGroup)
            }
            tabs.focus(oTab)
          }
        }

      } else if (click.middle) {
        tabs.close([oTab])

      } else if (click.right) {
        var a
        if (oTab.selected) {
          a = array.filter(oGroup.aTabs, function (x) {
            return x[info].selected
          })
          a = array.map(a, function (x) {
            return x[info]
          })
          assert(!!array.len(a))
          assert(array.indexOf(a, oTab) !== -1)
        } else {
          a = [oTab]
          deselectAllTabs(oGroup)
        }
        menus.tab.state.set({
          tabs: array.filter(a, function (x) {
            return x.visible
          })
        })
        ui.menu.show(menus.tab.menu, {
          left: click.mouseX + 5,
          top:  click.mouseY + 5
        })
      }
    }

    function getTabs(e) {
      if (e[info].selected) {
        return array.filter(e[group].aTabs, function (e) {
          return e[info].selected
        })
      } else {
        return [e]
      }
    }

    function moveTab(a, index, oTo) {
      var oGroup = oTo[group]
      //assert(oGroup.aTabs[index] === oTo)

      /*var len = array.len(a)
      array.each(a, function (x, i) {
        var oFrom = x[group]
          , oInfo = x[info]
          , curr  = array.indexOf(oFrom.aTabs, x)

        assert(curr !== -1)
        assert(oFrom.aTabs[curr] === x)

        x[group] = oGroup

        x.TITLE = oInfo.title

        assert(oFrom.oTabs[oInfo.id] === x)
        delete oFrom.oTabs[oInfo.id]
        assert(oGroup.oTabs[oInfo.id] == null)
        oGroup.oTabs[oInfo.id] = x

        array.removeAt(oFrom.aTabs, curr)

        if (/*oFrom.aTabs === oGroup.aTabs && *//*curr < index) {
          array.insertAt(oGroup.aTabs, index - 1, x)
          // TODO this seems a bit hacky...
          oInfo.active.index = index + i// - len
        } else {
          array.insertAt(oGroup.aTabs, index + i, x)
          oInfo.active.index = index + i
        }
      })

      //log(array.copy(oGroup.aTabs))

      var min = array.indexOf(oGroup.aTabs, a[0])
        , max = array.indexOf(oGroup.aTabs, array.last(a))
      assert(min !== -1)
      assert(max !== -1)
      array.each(array.slice(oGroup.aTabs, math.min(min, max) - 1,
                                    math.max(min, max) + 2), function (x) {
        log(x[info].title, x[info].active.index)
      })*/

      /*log(array.filter(oGroup.aTabs, function (x) {
        return array.some(a, function (y) {
          return x === y
        })
      }))*/

      tabs.move(array.map(a, function (x) {
        return x[info]
      }), index, oGroup.id)
    }

    function makeTab(tab) {
      return ui.tab.make(tab, {
        click: tabClick,
        getTabs: getTabs,
        move: moveTab
      })
    }


    function addTabTo(sort, tab, oGroup, animate) {
      var o = makeTab(tab)
      o[info]  = tab
      o[group] = oGroup
      oGroup.oTabs[tab.id] = o

      var a     = oGroup.aTabs
      var index = array.insertSorted(a, o, sort)
      var elem  = a[index + 1]
      if (elem == null) {
        o.move(oGroup.tabList)
      } else {
        o.moveBefore(elem)
      }

      if (animate) {
        ui.tab.show(o, 1)
      }
    }

    function removeTabFrom(tab, oGroup, animate) {
      // TODO use array.removeSorted ?
      array.remove(oGroup.aTabs, tab)
      if (animate) {
        ui.tab.hide(tab, 1)
      } else {
        tab.remove()
      }
    }

    function addTab(e, tab, animate) {
      var sort = groupSort.get().tabSort
      addGroups(e, tab, animate, function (oGroup) {
        assert(!(tab.id in oGroup.oTabs))
        addTabTo(sort, tab, oGroup, animate)
      })
    }

    function updateTab(e, tab, animate) {
      var sort = groupSort.get().tabSort

      var seen = {}
      addGroups(e, tab, animate, function (oGroup) {
        seen[oGroup.id] = true

        var old = oGroup.oTabs[tab.id]
        if (old != null) {
          assert(old[info].id === tab.id)
          assert(old[group] === oGroup)

          var b = (old[info].url  === tab.url &&
                   old[info].type === tab.type)

          old[info] = tab

          // Doesn't move the tab, just updates in place
          if (b && array.isElementSorted(oGroup.aTabs, old, sort)) {
            ui.tab.update(old, old[info])

          // Moves the tab
          } else {
            removeTabFrom(old, oGroup, animate)
            addTabTo(sort, tab, oGroup, animate)
          }

        } else {
          // Adds the tab
          addTabTo(sort, tab, oGroup, animate)
        }
      })

      removeTabIf(tab, animate, function (oGroup) {
        return !(oGroup.id in seen)
      })
    }

    // Does not add tabs, remove tabs, or change sort order
    // Only updates existing tabs without animation
    function updateWithoutMoving(tab) {
      array.each(aGroups, function (oGroup) {
        var o = oGroup.oTabs[tab.id]
        if (o != null) {
          o[info] = tab
          ui.tab.update(o, o[info])
        }
      })
    }

    function removeTab(tab) {
      removeTabIf(tab, true, function () {
        return true
      })
    }

    function init() {
      object.each(tabs.all.get(), function (tab) {
        addTab(e, tab, false)
      })
      searchTabs(search.value.get())
    }
    init()

    e.event([groupSort], function () {
      hide(e, 0.5, function () {
        reset()
        init()
        // TODO this is here for smoother animation
        setTimeout(function () {
          show(e, 0.5)
        }, 0)
      })
    })

    ;(function () {
      function add(x) {
        addTab(e, x, true)
      }
      function update(x) {
        updateTab(e, x, true)
      }
      function move(x) {
        updateTab(e, x, false)
      }
      function updateRaw(x) {
        updateWithoutMoving(x)
      }
      function remove(x) {
        removeTab(x)
      }

      //e.event([tabs.on.moved], log)
      //e.event([tabs.on.updateIndex], log)

      /*} else if (type === "windowName") {
          var sort = groupSort.get()
          array.each(aGroups, function (group) {
            group.name.set(sort.name(group))
          })*/

      e.event([tabs.on.opened], add)
      e.event([tabs.on.updated], update)
      e.event([tabs.on.moved], move)
      e.event([tabs.on.focused], update)
      e.event([tabs.on.updateIndex], updateRaw)
      e.event([tabs.on.unfocused], updateRaw)
      e.event([tabs.on.selected], updateRaw)
      e.event([tabs.on.deselected], updateRaw)
      e.event([tabs.on.closed], remove)
    })()

                           // TODO inefficient
    e.event([search.value, tabs.all], function (f) {
      searchTabs(f)
    })
  }
})
