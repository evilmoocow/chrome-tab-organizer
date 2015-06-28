import { each } from "../util/iterator";
import { async, concurrent } from "../util/async";
import { async_chrome } from "./common/util";
import { make_window } from "./server/windows";
import { make_popup } from "./server/popups";
import "./server/events";

// Exports
import { init_db } from "./server/db";
import { windows,
         open_window,
         event_window_open,
         event_window_close,
         event_window_focus } from "./server/windows";
import { event_tab_open,
         event_tab_close,
         event_tab_focus,
         event_tab_replace,
         event_tab_move,
         event_tab_update } from "./server/tabs";
import { on_connect,
         ports,
         send } from "./server/port";


// TODO do I need to wait for the "load" event before doing this ?
export const init_windows = async(function* () {
  const a = yield async_chrome((callback) => {
    chrome["windows"]["getAll"]({ "populate": true }, callback);
  });

  each(a, (info) => {
    make_window(info, false);
    make_popup(info, false);
  });
});

export const init = async(function* () {
  const db = yield init_db;

  // TODO change this to use the same system as `init_db`
  yield init_windows;

  return {
    db,

    windows,
    open_window,
    event_window_open,
    event_window_close,
    event_window_focus,

    event_tab_open,
    event_tab_close,
    event_tab_focus,
    event_tab_replace,
    event_tab_move,
    event_tab_update,

    on_connect,
    ports,
    send
  };
});
