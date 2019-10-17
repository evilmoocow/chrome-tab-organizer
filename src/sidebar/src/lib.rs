#![warn(unreachable_pub)]

use wasm_bindgen::prelude::*;
use wasm_bindgen::{intern, JsCast};
use std::sync::Arc;
use tab_organizer::{Timer, set_interval, connect, local_storage_set, log, time, generate_uuid, option_str, option_str_default, option_str_default_fn, is_empty, cursor, none_if, none_if_px, px, px_range, float_range, ease, TimeDifference, every_hour};
use tab_organizer::state as shared;
use tab_organizer::state::{SidebarMessage, BackgroundMessage, TabChange, Options, SortTabs};
use dominator::traits::*;
use dominator::{Dom, DomBuilder, text_signal, RefFn, html, stylesheet, clone, events, with_node, apply_methods};
use dominator::animation::{Percentage, MutableAnimation};
use js_sys::Date;
use futures::future::ready;
use futures::stream::{StreamExt, TryStreamExt};
use futures_signals::map_ref;
use futures_signals::signal::{Mutable, SignalExt, and, or};
use futures_signals::signal_vec::SignalVecExt;
use lazy_static::lazy_static;
use web_sys::{HtmlElement, window, ScrollRestoration, HtmlInputElement};

use crate::types::{State, DragState, Tab};
use crate::constants::*;

mod constants;
mod types;
mod search;
mod url_bar;
mod menu;
mod groups;
mod scrolling;
mod dragging;
mod tab;
mod culling;


// Whether it should automatically add/remove/update test tabs
const DYNAMIC_TAB_TEST: bool = false;


lazy_static! {
    static ref FAILED: Mutable<Option<Arc<String>>> = Mutable::new(None);

    static ref IS_LOADED: Mutable<bool> = Mutable::new(false);

    static ref SHOW_MODAL: Mutable<bool> = Mutable::new(false);
}


fn initialize(state: Arc<State>) {
    fn make_url_bar_child<A, D, F>(state: &State, name: &str, mut display: D, f: F) -> Dom
        where A: AsStr,
              D: FnMut(Arc<url_bar::UrlBar>) -> bool + 'static,
              F: FnMut(Option<Arc<url_bar::UrlBar>>) -> A + 'static {
        html!("div", {
            .class([
                &*URL_BAR_TEXT_STYLE,
                name,
            ])

            .visible_signal(state.url_bar.signal_cloned().map(move |url_bar| {
                if let Some(url_bar) = url_bar {
                    display(url_bar)

                } else {
                    false
                }
            }))

            .text_signal(state.url_bar.signal_cloned().map(f))
        })
    }

    fn tab_favicon<A>(tab: &Tab, mixin: A) -> Dom where A: FnOnce(DomBuilder<HtmlElement>) -> DomBuilder<HtmlElement> {
        html!("img", {
            .class(&*TAB_FAVICON_STYLE)
            /*.class([
                &*TAB_FAVICON_STYLE,
                &*ICON_STYLE,
            ])*/

            .class_signal(&*TAB_FAVICON_STYLE_UNLOADED, tab.unloaded.signal().first())

            .attribute_signal("src", tab.favicon_url.signal_cloned().map(|x| {
                RefFn::new(x, move |x| x.as_ref().map(|x| x.as_str()).unwrap_or(DEFAULT_FAVICON))
            }))

            .apply(mixin)
        })
    }

    fn tab_text<A>(tab: &Tab, mixin: A) -> Dom where A: FnOnce(DomBuilder<HtmlElement>) -> DomBuilder<HtmlElement> {
        html!("div", {
            .class([
                &*STRETCH_STYLE,
                &*TAB_TEXT_STYLE,
            ])

            .children(&mut [
                html!("span", {
                    .children(&mut [
                        text_signal(map_ref! {
                            let title = tab.title.signal_cloned(),
                            let unloaded = tab.unloaded.signal() => {
                                if *unloaded {
                                    if title.is_some() {
                                        "➔ "

                                    } else {
                                        "➔"
                                    }

                                } else {
                                    ""
                                }
                            }
                        }.first()),

                        text_signal(tab.title.signal_cloned().map(|x| option_str_default(x, "")).first()),
                    ])
                })
            ])

            .apply(mixin)
        })
    }

    fn tab_close<A>(mixin: A) -> Dom where A: FnOnce(DomBuilder<HtmlElement>) -> DomBuilder<HtmlElement> {
        html!("div", {
            .class(&*TAB_CLOSE_STYLE)

            .children(&mut [
                html!("img", {
                    .class(&*TAB_CLOSE_ICON_STYLE)
                    /*.class([
                        &*TAB_CLOSE_STYLE,
                        &*ICON_STYLE,
                    ])*/

                    .attribute("src", "data/images/button-close.png")
                }),
            ])

            .apply(mixin)
        })
    }

    fn tab_template<A>(state: &State, tab: &Tab, favicon: Dom, text: Dom, close: Dom, mixin: A) -> Dom
        where A: FnOnce(DomBuilder<HtmlElement>) -> DomBuilder<HtmlElement> {

        html!("div", {
            .class([
                &*ROW_STYLE,
                &*TAB_STYLE,
                //&*MENU_ITEM_STYLE,
            ])

            .cursor!(state.is_dragging(), intern("pointer"))

            .class_signal(&*TAB_UNLOADED_STYLE, tab.unloaded.signal().first())
            .class_signal(&*TAB_FOCUSED_STYLE, tab.is_focused())

            .children(&mut [favicon, text, close])

            .apply(mixin)
        })
    }


    stylesheet!("html, body", {
        .style_signal("cursor", state.is_dragging().map(|is_dragging| {
            if is_dragging {
                Some("grabbing")

            } else {
                None
            }
        }))
    });

    let window_height = Mutable::new(tab_organizer::window_height());

    dominator::append_dom(&dominator::body(),
        html!("div", {
            .class(&*TOP_STYLE)

            // TODO only attach this when dragging
            .global_event(clone!(state => move |_: events::MouseUp| {
                state.drag_end();
            }))

            // TODO only attach this when dragging
            .global_event(clone!(state => move |e: events::MouseMove| {
                state.drag_move(e.mouse_x(), e.mouse_y());
            }))

            .future(culling::cull_groups(state.clone(), window_height.signal()))

            .global_event(move |_: events::Resize| {
                window_height.set_neq(tab_organizer::window_height());
            })

            .children(&mut [
                html!("div", {
                    .class(&*DRAGGING_STYLE)

                    .visible_signal(state.is_dragging())

                    .style_signal("width", state.dragging.state.signal_ref(|dragging| {
                        if let Some(DragState::Dragging { rect, .. }) = dragging {
                            Some(px(rect.width()))

                        } else {
                            None
                        }
                    }))

                    .style_signal("transform", state.dragging.state.signal_ref(|dragging| {
                        if let Some(DragState::Dragging { mouse_y, rect, .. }) = dragging {
                            Some(format!("translate({}px, {}px)", rect.x().round(), (mouse_y - TAB_DRAGGING_TOP)))

                        } else {
                            None
                        }
                    }))

                    .children_signal_vec(state.dragging.selected_tabs.signal_ref(clone!(state => move |tabs| {
                        tabs.iter().enumerate().map(|(index, tab)| {
                            // TODO use some sort of oneshot animation instead
                            // TODO don't create the animation at all for index 0
                            let animation = MutableAnimation::new(SELECTED_TABS_ANIMATION_DURATION);

                            if index > 0 {
                                animation.animate_to(Percentage::new(1.0));
                            }

                            Dom::with_state(animation, |animation| {
                                tab_template(&state, &tab,
                                    tab_favicon(&tab, |dom| dom),
                                    tab_text(&tab, |dom| dom),

                                    if index == 0 {
                                        tab_close(|dom| dom)

                                    } else {
                                        dominator::Dom::empty()
                                    },

                                    |dom| dom
                                        .class_signal(&*TAB_SELECTED_STYLE, tab.selected.signal())
                                        .class(&*MENU_ITEM_SHADOW_STYLE)
                                        .style("z-index", format!("-{}", index))

                                        .apply_if(index == 0, |dom| dom
                                            .class(&*TAB_HOVER_STYLE)
                                            /*.class([
                                                &*TAB_HOVER_STYLE,
                                                &*MENU_ITEM_HOVER_STYLE,
                                            ])*/
                                            .class_signal(&*TAB_SELECTED_HOVER_STYLE, tab.selected.signal())
                                            .class_signal(&*TAB_UNLOADED_HOVER_STYLE, tab.unloaded.signal()) // TODO use .first() ?
                                            .class_signal(&*TAB_FOCUSED_HOVER_STYLE, tab.is_focused()))

                                        // TODO use ease-out easing
                                        .apply_if(index > 0 && index < 5, |dom| dom
                                            .style_signal("margin-top", none_if(animation.signal(), 0.0, px_range, 0.0, -(TAB_TOTAL_HEIGHT - 2.0))))

                                        .apply_if(index >= 5, |dom| dom
                                            .style_signal("margin-top", none_if(animation.signal(), 0.0, px_range, 0.0, -TAB_TOTAL_HEIGHT))
                                            // TODO use ease-out easing
                                            .style_signal("opacity", none_if(animation.signal(), 0.0, float_range, 1.0, 0.0))))
                            })
                        }).collect()
                    })).to_signal_vec())
                }),

                html!("div", {
                    .class([
                        &*ROW_STYLE,
                        &*URL_BAR_STYLE,
                    ])

                    .visible_signal(map_ref! {
                        let is_dragging = state.is_dragging(),
                        let url_bar = state.url_bar.signal_cloned() => {
                            // TODO a bit hacky
                            let matches = url_bar.as_ref().map(|url_bar| {
                                !is_empty(&url_bar.protocol) ||
                                !is_empty(&url_bar.domain) ||
                                !is_empty(&url_bar.path) ||
                                !is_empty(&url_bar.file) ||
                                !is_empty(&url_bar.query) ||
                                !is_empty(&url_bar.hash)
                            }).unwrap_or(false);

                            !is_dragging && matches
                        }
                    })

                    // TODO check if any of these need "flex-shrink": 1
                    .children(&mut [
                        make_url_bar_child(&state, &URL_BAR_PROTOCOL_STYLE, |x| !is_empty(&x.protocol), |url_bar| option_str_default_fn(url_bar, "", |x| &x.protocol)), // .as_ref().map(|x| x.as_str())
                        make_url_bar_child(&state, &URL_BAR_DOMAIN_STYLE, |x| !is_empty(&x.domain), |url_bar| option_str_default_fn(url_bar, "", |x| &x.domain)),
                        make_url_bar_child(&state, &URL_BAR_PATH_STYLE, |x| !is_empty(&x.path), |url_bar| option_str_default_fn(url_bar, "", |x| &x.path)),
                        make_url_bar_child(&state, &URL_BAR_FILE_STYLE, |x| !is_empty(&x.file), |url_bar| option_str_default_fn(url_bar, "", |x| &x.file)),
                        make_url_bar_child(&state, &URL_BAR_QUERY_STYLE, |x| !is_empty(&x.query), |url_bar| option_str_default_fn(url_bar, "", |x| &x.query)),
                        make_url_bar_child(&state, &URL_BAR_HASH_STYLE, |x| !is_empty(&x.hash), |url_bar| option_str_default_fn(url_bar, "", |x| &x.hash)),
                    ])
                }),

                html!("div", {
                    .class([
                        &*ROW_STYLE,
                        &*TOOLBAR_STYLE,
                    ])

                    .children(&mut [
                        html!("input" => HtmlInputElement, {
                            .class([
                                &*SEARCH_STYLE,
                                &*STRETCH_STYLE,
                            ])

                            .cursor!(state.is_dragging(), "auto")

                            .style_signal("background-color", FAILED.signal_cloned().map(|failed| {
                                if failed.is_some() {
                                    Some("hsl(5, 100%, 90%)")

                                } else {
                                    None
                                }
                            }))

                            .attribute("type", "text")
                            .attribute("autofocus", "")
                            .attribute("autocomplete", "off")
                            .attribute("placeholder", "Search")

                            .attribute_signal("title", FAILED.signal_cloned().map(|x| option_str_default(x, "")))

                            .attribute_signal("value", state.search_box.signal_cloned().map(|x| RefFn::new(x, |x| x.as_str())))

                            .with_node!(element => {
                                .event(clone!(state => move |_: events::Input| {
                                    let value = Arc::new(element.value());
                                    local_storage_set("tab-organizer.search", &value);
                                    // TODO is it faster to not use Arc ?
                                    state.search_parser.set(Arc::new(search::Parsed::new(&value)));
                                    state.search_box.set(value);
                                }))
                            })
                        }),

                        html!("div", {
                            .class(&*TOOLBAR_SEPARATOR_STYLE)
                        }),

                        {
                            let hovering = Mutable::new(false);
                            let holding = Mutable::new(false);

                            html!("div", {
                                .class(&*TOOLBAR_MENU_WRAPPER_STYLE)
                                .children(&mut [
                                    html!("div", {
                                        .class([
                                            &*ROW_STYLE,
                                            &*TOOLBAR_MENU_STYLE,
                                        ])

                                        .cursor!(state.is_dragging(), "pointer")

                                        .class_signal(&*TOOLBAR_MENU_HOLD_STYLE, and(hovering.signal(), holding.signal()))

                                        .event(clone!(hovering => move |_: events::MouseEnter| {
                                            hovering.set_neq(true);
                                        }))

                                        .event(move |_: events::MouseLeave| {
                                            hovering.set_neq(false);
                                        })

                                        .event(clone!(holding => move |_: events::MouseDown| {
                                            holding.set_neq(true);
                                        }))

                                        // TODO only attach this when holding
                                        .global_event(move |_: events::MouseUp| {
                                            holding.set_neq(false);
                                        })

                                        .event(clone!(state => move |_: events::Click| {
                                            state.menu.show();
                                        }))

                                        .text("Menu")
                                    }),

                                    state.menu.render(|menu| { menu
                                        .submenu("Sort tabs by...", |menu| { menu
                                            .option("Window", state.options.sort_tabs.signal_ref(|x| *x == SortTabs::Window), clone!(state => move || {
                                                state.options.sort_tabs.set_neq(SortTabs::Window);
                                            }))

                                            .option("Tag", state.options.sort_tabs.signal_ref(|x| *x == SortTabs::Tag), clone!(state => move || {
                                                state.options.sort_tabs.set_neq(SortTabs::Tag);
                                            }))

                                            .separator()

                                            .option("Time (focused)", state.options.sort_tabs.signal_ref(|x| *x == SortTabs::TimeFocused), clone!(state => move || {
                                                state.options.sort_tabs.set_neq(SortTabs::TimeFocused);
                                            }))

                                            .option("Time (created)", state.options.sort_tabs.signal_ref(|x| *x == SortTabs::TimeCreated), clone!(state => move || {
                                                state.options.sort_tabs.set_neq(SortTabs::TimeCreated);
                                            }))

                                            .separator()

                                            .option("URL", state.options.sort_tabs.signal_ref(|x| *x == SortTabs::Url), clone!(state => move || {
                                                state.options.sort_tabs.set_neq(SortTabs::Url);
                                            }))

                                            .option("Name", state.options.sort_tabs.signal_ref(|x| *x == SortTabs::Name), clone!(state => move || {
                                                state.options.sort_tabs.set_neq(SortTabs::Name);
                                            }))
                                        })

                                        .separator()

                                        .submenu("Foo", |menu| { menu
                                            .option("Bar", futures_signals::signal::always(true), || {})
                                            .option("Qux", futures_signals::signal::always(false), || {})
                                        })
                                    }),
                                ])
                            })
                        },
                    ])
                }),

                html!("div", {
                    .class(&*GROUP_LIST_STYLE)

                    .event_preventable(move |e: events::MouseDown| {
                        e.prevent_default();
                    })

                    .with_node!(element => {
                        // TODO also update these when groups/tabs are added/removed ?
                        .event(clone!(state, element => move |_: events::Scroll| {
                            if IS_LOADED.get() {
                                let y = element.scroll_top() as f64;
                                // TODO is there a more efficient way of converting to a string ?
                                local_storage_set("tab-organizer.scroll.y", &y.to_string());
                                state.scrolling.y.set_neq(y);
                            }
                        }))

                        // TODO use set_scroll_top instead
                        .future(map_ref! {
                            let loaded = IS_LOADED.signal(),
                            let scroll_y = state.scrolling.y.signal() => {
                                if *loaded {
                                    Some(*scroll_y)

                                } else {
                                    None
                                }
                            }
                        // TODO super hacky, figure out a better way to keep the scroll_y in bounds
                        }.for_each(clone!(state => move |scroll_y| {
                            if let Some(scroll_y) = scroll_y {
                                let scroll_y = scroll_y.round();
                                let old_scroll_y = element.scroll_top() as f64;

                                if old_scroll_y != scroll_y {
                                    element.set_scroll_top(scroll_y as i32);

                                    // TODO does this cause a reflow ?
                                    let new_scroll_y = element.scroll_top() as f64;

                                    if new_scroll_y != scroll_y {
                                        state.scrolling.y.set_neq(new_scroll_y);
                                    }
                                }
                            }

                            ready(())
                        })))
                    })

                    .children(&mut [
                        // TODO this is pretty hacky, but I don't know a better way to make it work
                        html!("div", {
                            .class(&*GROUP_LIST_CHILDREN_STYLE)

                            .style_signal("padding-top", state.groups_padding.signal().map(none_if_px(0.0)))
                            .style_signal("height", state.scrolling.height.signal().map(none_if_px(0.0)))

                            .children_signal_vec(state.groups.signal_vec_cloned().enumerate()
                                .delay_remove(|(_, group)| group.wait_until_removed())
                                .filter_signal_cloned(|(_, group)| group.visible.signal())
                                .map(clone!(state => move |(index, group)| {
                                    if let Some(index) = index.get() {
                                        if state.should_be_dragging_group(index) {
                                            group.drag_top.jump_to(Percentage::new(1.0));
                                        }
                                    }

                                    html!("div", {
                                        .class(&*GROUP_STYLE)

                                        .style_signal("top", none_if(group.drag_top.signal(), 0.0, px_range, -1.0, DRAG_GAP_PX - 1.0))
                                        .style_signal("padding-bottom", none_if(group.drag_over.signal(), 0.0, px_range, 0.0, DRAG_GAP_PX))
                                        .style_signal("margin-bottom", none_if(group.drag_over.signal(), 0.0, px_range, 0.0, -DRAG_GAP_PX))

                                        .style_signal("padding-top", none_if(group.insert_animation.signal(), 1.0, px_range, 0.0, GROUP_PADDING_TOP))
                                        .style_signal("border-top-width", none_if(group.insert_animation.signal(), 1.0, px_range, 0.0, GROUP_BORDER_WIDTH))
                                        .style_signal("opacity", none_if(group.insert_animation.signal(), 1.0, float_range, 0.0, 1.0))

                                        .event(clone!(state, group, index => move |_: events::MouseEnter| {
                                            if let Some(index) = index.get() {
                                                state.drag_over_group(group.clone(), index);
                                            }
                                        }))

                                        .children(&mut [
                                            if group.show_header {
                                                html!("div", {
                                                    .class([
                                                        &*ROW_STYLE,
                                                        &*GROUP_HEADER_STYLE,
                                                    ])

                                                    .style_signal("height", none_if(group.insert_animation.signal(), 1.0, px_range, 0.0, GROUP_HEADER_HEIGHT))
                                                    .style_signal("margin-left", none_if(group.insert_animation.signal(), 1.0, px_range, INSERT_LEFT_MARGIN, 0.0))

                                                    .children(&mut [
                                                        html!("div", {
                                                            .class([
                                                                &*GROUP_HEADER_TEXT_STYLE,
                                                                &*STRETCH_STYLE,
                                                            ])
                                                            .text_signal(map_ref! {
                                                                    let name = group.name.signal_cloned(),
                                                                    let index = index.signal() => {
                                                                        // TODO improve the efficiency of this ?
                                                                        name.clone().or_else(|| {
                                                                            index.map(|index| Arc::new((index + 1).to_string()))
                                                                        })
                                                                    }
                                                                }
                                                                // This causes it to remember the previous value if it returns `None`
                                                                // TODO dedicated method for this ?
                                                                .filter_map(|x| x)
                                                                .map(|x| option_str_default(x, "")))
                                                        }),
                                                    ])
                                                })

                                            } else {
                                                Dom::empty()
                                            },

                                            html!("div", {
                                                .class(&*GROUP_TABS_STYLE)

                                                .style_signal("padding-top", group.tabs_padding.signal().map(none_if_px(0.0)))
                                                .style_signal("padding-bottom", none_if(group.insert_animation.signal(), 1.0, px_range, 0.0, GROUP_PADDING_BOTTOM))

                                                .children_signal_vec(group.tabs.signal_vec_cloned().enumerate()
                                                    .delay_remove(|(_, tab)| tab.wait_until_removed())
                                                    .filter_signal_cloned(|(_, tab)| tab.visible.signal())
                                                    .map(clone!(state => move |(index, tab)| {
                                                        if let Some(index) = index.get() {
                                                            if state.should_be_dragging_tab(group.id, index) {
                                                                tab.drag_over.jump_to(Percentage::new(1.0));
                                                            }
                                                        }

                                                        tab_template(&state, &tab,
                                                            tab_favicon(&tab, |dom| { dom
                                                                .style_signal("height", none_if(tab.insert_animation.signal(), 1.0, px_range, 0.0, TAB_FAVICON_SIZE))
                                                            }),

                                                            tab_text(&tab, |dom| { dom }),

                                                            tab_close(|dom| { dom
                                                                .class_signal(&*TAB_CLOSE_HOVER_STYLE, tab.close_hovered.signal())
                                                                .class_signal(&*TAB_CLOSE_HOLD_STYLE, and(tab.close_hovered.signal(), tab.close_holding.signal()))

                                                                .style_signal("height", none_if(tab.insert_animation.signal(), 1.0, px_range, 0.0, TAB_FAVICON_SIZE))
                                                                .style_signal("border-top-width", none_if(tab.insert_animation.signal(), 1.0, px_range, 0.0, TAB_CLOSE_BORDER_WIDTH))
                                                                .style_signal("border-bottom-width", none_if(tab.insert_animation.signal(), 1.0, px_range, 0.0, TAB_CLOSE_BORDER_WIDTH))

                                                                .visible_signal(state.is_tab_hovered(&tab))

                                                                .event(clone!(tab => move |_: events::MouseEnter| {
                                                                    tab.close_hovered.set_neq(true);
                                                                }))

                                                                .event(clone!(tab => move |_: events::MouseLeave| {
                                                                    tab.close_hovered.set_neq(false);
                                                                }))

                                                                .event(clone!(tab => move |_: events::MouseDown| {
                                                                    tab.close_holding.set_neq(true);
                                                                }))

                                                                // TODO only attach this when hovering
                                                                .global_event(clone!(tab => move |_: events::MouseUp| {
                                                                    tab.close_holding.set_neq(false);
                                                                }))

                                                                .event(clone!(state, tab => move |_: events::Click| {
                                                                    state.close_tab(&tab);
                                                                }))
                                                            }),

                                                            |dom| apply_methods!(dom, {
                                                                .class_signal(&*TAB_HOVER_STYLE, state.is_tab_hovered(&tab))
                                                                //.class_signal(&*MENU_ITEM_HOVER_STYLE, state.is_tab_hovered(&tab))
                                                                .class_signal(&*TAB_UNLOADED_HOVER_STYLE, and(state.is_tab_hovered(&tab), tab.unloaded.signal().first()))
                                                                .class_signal(&*TAB_FOCUSED_HOVER_STYLE, and(state.is_tab_hovered(&tab), tab.is_focused()))

                                                                //.class_signal(&*TAB_HOLD_STYLE, state.is_tab_holding(&tab))
                                                                //.class_signal(&*MENU_ITEM_HOLD_STYLE, state.is_tab_holding(&tab))

                                                                .class_signal(&*TAB_SELECTED_STYLE, tab.selected.signal())
                                                                .class_signal(&*TAB_SELECTED_HOVER_STYLE, and(state.is_tab_hovered(&tab), tab.selected.signal()))
                                                                .class_signal(&*MENU_ITEM_SHADOW_STYLE, or(tab.is_focused(), tab.selected.signal()))

                                                                .attribute_signal("title", tab.title.signal_cloned().map(|x| option_str_default(x, "")).first())

                                                                .style_signal("margin-left", none_if(tab.insert_animation.signal(), 1.0, px_range, INSERT_LEFT_MARGIN, 0.0))
                                                                .style_signal("height", none_if(tab.insert_animation.signal(), 1.0, px_range, 0.0, TAB_HEIGHT))
                                                                .style_signal("padding-top", none_if(tab.insert_animation.signal(), 1.0, px_range, 0.0, TAB_PADDING))
                                                                .style_signal("padding-bottom", none_if(tab.insert_animation.signal(), 1.0, px_range, 0.0, TAB_PADDING))
                                                                .style_signal("border-top-width", none_if(tab.insert_animation.signal(), 1.0, px_range, 0.0, TAB_BORDER_WIDTH))
                                                                .style_signal("border-bottom-width", none_if(tab.insert_animation.signal(), 1.0, px_range, 0.0, TAB_BORDER_WIDTH))
                                                                .style_signal("opacity", none_if(tab.insert_animation.signal(), 1.0, float_range, 0.0, 1.0))

                                                                .style_signal("transform", tab.insert_animation.signal().map(|t| {
                                                                    t.none_if(1.0).map(|t| format!("rotateX({}deg)", ease(t).range_inclusive(-90.0, 0.0)))
                                                                }))

                                                                .style_signal("top", none_if(tab.drag_over.signal(), 0.0, px_range, 0.0, DRAG_GAP_PX))

                                                                .with_node!(element => {
                                                                    .event(clone!(state, index, group, tab => move |e: events::MouseDown| {
                                                                        // TODO a little hacky
                                                                        if !tab.close_hovered.get() {
                                                                            //tab.holding.set_neq(true);

                                                                            if let Some(index) = index.get() {
                                                                                let shift = e.shift_key();
                                                                                // TODO is this correct ?
                                                                                // TODO test this, especially on Mac
                                                                                let ctrl = e.ctrl_key();
                                                                                let alt = e.alt_key();

                                                                                if let events::MouseButton::Left = e.button() {
                                                                                    // TODO a little hacky
                                                                                    if ctrl && !shift && !alt {
                                                                                        group.ctrl_select_tab(&tab);

                                                                                    } else if !ctrl && shift && !alt {
                                                                                        group.shift_select_tab(&tab);

                                                                                    } else if !ctrl && !shift && !alt {
                                                                                        state.click_tab(&group, &tab);

                                                                                        let rect = element.get_bounding_client_rect();
                                                                                        state.drag_start(e.mouse_x(), e.mouse_y(), rect, group.clone(), tab.clone(), index);
                                                                                    }
                                                                                }
                                                                            }
                                                                        }
                                                                    }))
                                                                })

                                                                // TODO only attach this when holding
                                                                /*.global_event(clone!(tab => move |_: events::MouseUp| {
                                                                    tab.holding.set_neq(false);
                                                                }))*/

                                                                .event(clone!(state, index, group, tab => move |_: events::MouseEnter| {
                                                                    // TODO should this be inside of the if ?
                                                                    state.hover_tab(&tab);

                                                                    if let Some(index) = index.get() {
                                                                        state.drag_over(group.clone(), index);
                                                                    }
                                                                }))

                                                                .event(clone!(state, tab => move |_: events::MouseLeave| {
                                                                    // TODO should this check the index, like MouseEnterEvent ?
                                                                    state.unhover_tab(&tab);
                                                                }))

                                                                // TODO replace with MouseClickEvent
                                                                .event(clone!(state, index, tab => move |e: events::MouseUp| {
                                                                    if index.get().is_some() {
                                                                        let shift = e.shift_key();
                                                                        // TODO is this correct ?
                                                                        // TODO test this, especially on Mac
                                                                        let ctrl = e.ctrl_key();
                                                                        let alt = e.alt_key();

                                                                        match e.button() {
                                                                            events::MouseButton::Left => {

                                                                            },
                                                                            events::MouseButton::Middle => {
                                                                                if !shift && !ctrl && !alt {
                                                                                    state.close_tab(&tab);
                                                                                }
                                                                            },
                                                                            events::MouseButton::Right => {
                                                                            },
                                                                            _ => {},
                                                                        }
                                                                    }
                                                                }))
                                                            })
                                                        )
                                                    })))
                                            }),
                                        ])
                                    })
                                })))
                        }),

                        html!("div", {
                            .class(&*GROUP_LIST_RIGHT_BORDER)
                        }),
                    ])
                }),
            ])
        }),
    );

    every_hour(clone!(state => move || {
        time!("Updating group titles", {
            state.update_group_titles();
        });
    }));

    // TODO a little hacky, needed to ensure that scrolling happens after everything is created
    window()
        .unwrap_throw()
        .request_animation_frame(Closure::once_into_js(move |_: f64| {
            IS_LOADED.set_neq(true);
            SHOW_MODAL.set_neq(false);
            log!("Loaded");
        }).unchecked_ref())
        .unwrap_throw();

    log!("Finished");

    /*let mut tag_counter = 0;

    if DYNAMIC_TAB_TEST {
        set_interval(clone!(state => move || {
            state.process_message(BackgroundMessage::TabChanged {
                tab_index: 2,
                changes: vec![
                    TabChange::Title {
                        new_title: Some(generate_uuid().to_string()),
                    },
                ],
            });

            state.process_message(BackgroundMessage::TabChanged {
                tab_index: 3,
                changes: vec![
                    TabChange::Title {
                        new_title: Some("e1".to_string()),
                    },
                ],
            });

            state.process_message(BackgroundMessage::TabChanged {
                tab_index: 3,
                changes: vec![
                    TabChange::Title {
                        new_title: Some("e2".to_string()),
                    },
                ],
            });

            /*state.process_message(BackgroundMessage::TabChanged {
                tab_index: 0,
                changes: vec![
                    TabChange::Pinned {
                        pinned: false,
                    },
                ],
            });*/

            state.process_message(BackgroundMessage::TabRemoved {
                tab_index: 0,
            });

            state.process_message(BackgroundMessage::TabRemoved {
                tab_index: 0,
            });

            state.process_message(BackgroundMessage::TabRemoved {
                tab_index: 8,
            });

            /*state.process_message(BackgroundMessage::TabInserted {
                tab_index: 0,
                tab: shared::Tab {
                    serialized: shared::SerializedTab {
                        id: generate_uuid(),
                        timestamp_created: Date::now(),
                        timestamp_focused: Date::now(),
                    },
                    focused: false,
                    unloaded: true,
                    pinned: true,
                    favicon_url: Some("http://www.saltybet.com/favicon.ico".to_owned()),
                    url: Some("top".to_owned()),
                    title: Some("top".to_owned()),
                },
            });*/

            let timestamp = Date::now();

            state.process_message(BackgroundMessage::TabInserted {
                tab_index: 12,
                tab: shared::Tab {
                    serialized: shared::SerializedTab {
                        id: generate_uuid(),
                        timestamp_created: timestamp,
                        timestamp_focused: timestamp,
                        tags: vec![
                            shared::Tag {
                                name: "New".to_string(),
                                timestamp_added: Date::now(),
                            },
                        ],
                    },
                    focused: false,
                    unloaded: true,
                    pinned: false,
                    favicon_url: Some("http://www.saltybet.com/favicon.ico".to_owned()),
                    url: Some("bottom".to_owned()),
                    title: Some(format!("bottom {}", timestamp)),
                },
            });

            state.process_message(BackgroundMessage::TabInserted {
                tab_index: 13,
                tab: shared::Tab {
                    serialized: shared::SerializedTab {
                        id: generate_uuid(),
                        timestamp_created: timestamp,
                        timestamp_focused: timestamp,
                        tags: vec![],
                    },
                    focused: false,
                    unloaded: true,
                    pinned: false,
                    favicon_url: Some("http://www.saltybet.com/favicon.ico".to_owned()),
                    url: Some("bottom".to_owned()),
                    title: Some(format!("bottom {}", timestamp)),
                },
            });

            state.process_message(BackgroundMessage::TabChanged {
                tab_index: 10,
                changes: vec![
                    TabChange::AddedToTag {
                        tag: shared::Tag {
                            name: tag_counter.to_string(),
                            timestamp_added: Date::now(),
                        },
                    },
                ],
            });

            tag_counter += 1;

            /*for _ in 0..10 {
                state.process_message(BackgroundMessage::TabRemoved {
                    window_index: 2,
                    tab_index: 0,
                });
            }

            state.process_message(BackgroundMessage::WindowRemoved {
                window_index: 2,
            });

            state.process_message(BackgroundMessage::WindowInserted {
                window_index: 2,
                window: shared::Window {
                    serialized: shared::SerializedWindow {
                        id: generate_uuid(),
                        name: None,
                        timestamp_created: Date::now(),
                        timestamp_focused: Date::now(),
                    },
                    focused: false,
                    tabs: vec![],
                },
            });

            for index in 0..10 {
                state.process_message(BackgroundMessage::TabInserted {
                    window_index: 2,
                    tab_index: index,
                    tab: shared::Tab {
                        serialized: shared::SerializedTab {
                            id: generate_uuid(),
                            timestamp_created: Date::now(),
                            timestamp_focused: Date::now(),
                        },
                        focused: index == 7,
                        unloaded: index == 5,
                        pinned: index == 0 || index == 1 || index == 2,
                        favicon_url: Some("http://www.saltybet.com/favicon.ico".to_owned()),
                        url: Some("https://www.example.com/foo?bar#qux".to_owned()),
                        title: Some("Foo".to_owned()),
                    },
                });
            }*/
        }), (INSERT_ANIMATION_DURATION * 2.0) as u32);

        set_interval(move || {
            state.process_message(BackgroundMessage::TabInserted {
                tab_index: 0,
                tab: shared::Tab {
                    serialized: shared::SerializedTab {
                        id: generate_uuid(),
                        timestamp_created: Date::now(),
                        timestamp_focused: Date::now(),
                        tags: vec![
                            shared::Tag {
                                name: "New (Pinned)".to_string(),
                                timestamp_added: Date::now(),
                            },
                        ],
                    },
                    focused: false,
                    unloaded: true,
                    pinned: true,
                    favicon_url: Some("http://www.saltybet.com/favicon.ico".to_owned()),
                    url: Some("top".to_owned()),
                    title: Some("top".to_owned()),
                },
            });

            state.process_message(BackgroundMessage::TabInserted {
                tab_index: 0,
                tab: shared::Tab {
                    serialized: shared::SerializedTab {
                        id: generate_uuid(),
                        timestamp_created: Date::now(),
                        timestamp_focused: Date::now(),
                        tags: vec![
                            shared::Tag {
                                name: "New (Pinned)".to_string(),
                                timestamp_added: Date::now(),
                            },
                        ],
                    },
                    focused: false,
                    unloaded: true,
                    pinned: true,
                    favicon_url: Some("http://www.saltybet.com/favicon.ico".to_owned()),
                    url: Some("top test".to_owned()),
                    title: Some("top test".to_owned()),
                },
            });
        }, (INSERT_ANIMATION_DURATION * 3.0) as u32);
    }*/
}


#[wasm_bindgen(start)]
pub fn main_js() {
    #[cfg(debug_assertions)]
    std::panic::set_hook(Box::new(move |info| {
    	let message = Arc::new(info.to_string());
        FAILED.set(Some(message.clone()));
        console_error_panic_hook::hook(info);
    }));


    log!("Starting");

    stylesheet!("*", {
        .style("text-overflow", "ellipsis")
        .style("box-sizing", "content-box")

        .style("vertical-align", "middle") /* TODO I can probably get rid of this */

        /* TODO is this correct ?*/
        .style("background-repeat", "no-repeat")
        .style("background-size", "100% 100%")
        .style("cursor", "inherit")
        .style("position", "relative")

        /* TODO are these a good idea ? */
        .style("outline-width", "0px")
        .style("outline-color", "transparent")
        .style("outline-style", "solid")

        .style("border-width", "0px")
        .style("border-color", "transparent")
        .style("border-style", "solid")

        .style("margin", "0px")
        .style("padding", "0px")

        .style("background-color", "transparent")

        .style("flex-shrink", "0") /* 1 */
        .style("flex-grow", "0") /* 1 */
        .style("flex-basis", "auto") /* 0% */ /* TODO try out other stuff like min-content once it becomes available */
    });

    stylesheet!("html, body", {
        .style("width", "100%")
        .style("height", "100%")

        .style(["-moz-user-select", "user-select"], "none")

        //.style("font-family", "message-box")
        .style("font-size", "13px")

        //.style("background-color", "hsl(0, 0%, 100%)")
        /*.style("background-image", "repeating-linear-gradient(0deg, \
                                        transparent                0px, \
                                        hsla(200, 30%, 30%, 0.017) 2px, \
                                        hsla(200, 30%, 30%, 0.017) 3px)")*/
        .style("background-color", "#fdfeff") // rgb(245, 246, 247) rgb(227, 228, 230)
    });

    // Disables the browser scroll restoration
    window()
        .unwrap_throw()
        .history()
        .unwrap_throw()
        .set_scroll_restoration(ScrollRestoration::Manual)
        .unwrap_throw();

    dominator::append_dom(&dominator::body(), html!("div", {
        .class([
            &*TOP_STYLE,
            &*MODAL_STYLE,
            &*CENTER_STYLE,
            &*LOADING_STYLE,
        ])

        .visible_signal(SHOW_MODAL.signal())

        .text("LOADING...")
    }));


    Timer::new(LOADING_MESSAGE_THRESHOLD, move || {
        if !IS_LOADED.get() {
            SHOW_MODAL.set_neq(true);
        }
    }).forget();


    fn search_to_id() -> String {
        let search = window()
                .unwrap_throw()
                .location()
                .search()
                .unwrap_throw();

        js_sys::decode_uri_component(&search[1..])
            .unwrap_throw()
            .into()
    }


    tab_organizer::spawn(async move {
        let port = connect("sidebar");

        port.send_message(&SidebarMessage::Initialize {
            id: search_to_id(),
        });

        let _ = port.on_message()
            .map(|x| -> Result<BackgroundMessage, JsValue> { Ok(x) })
            .try_fold(None, move |mut state, message| {
                clone!(port => async move {
                    match message {
                        BackgroundMessage::Initial { tabs } => {
                            state = time!("Initializing", {
                                let state = Arc::new(State::new(port, Options::new(), tabs));
                                initialize(state.clone());
                                Some(state)
                            });
                        },

                        BackgroundMessage::TabInserted { tab_index, tab } => {
                            state.as_ref().unwrap_throw().insert_tab(tab_index, tab);
                        },

                        BackgroundMessage::TabRemoved { tab_index } => {
                            state.as_ref().unwrap_throw().remove_tab(tab_index);
                        },

                        BackgroundMessage::TabChanged { tab_index, changes } => {
                            state.as_ref().unwrap_throw().change_tab(tab_index, changes);
                        },

                        BackgroundMessage::TabFocused { old_tab_index, new_tab_index, new_timestamp_focused } => {
                            state.as_ref().unwrap_throw().focus_tab(old_tab_index, new_tab_index, new_timestamp_focused);
                        },

                        BackgroundMessage::TabMoved { old_tab_index, new_tab_index } => {
                            state.as_ref().unwrap_throw().move_tab(old_tab_index, new_tab_index);
                        },
                    }

                    Ok(state)
                })
            }).await?;

        Ok(())
    });

    /*Timer::new(1500, move || {
        let window: shared::Window = shared::Window {
            serialized: shared::SerializedWindow {
                id: generate_uuid(),
                name: None,
                timestamp_created: Date::now(),
            },
            focused: false,
            tabs: (0..1000).map(|index| {
                shared::Tab {
                    serialized: shared::SerializedTab {
                        id: generate_uuid(),
                        timestamp_created: Date::now() - (index as f64 * TimeDifference::HOUR),
                        timestamp_focused: Date::now() - (index as f64 * TimeDifference::HOUR),
                        tags: vec![
                            shared::Tag {
                                name: if index < 5 { "One".to_string() } else { "Two".to_string() },
                                timestamp_added: index as f64,
                            },
                        ],
                    },
                    focused: index == 7,
                    unloaded: index == 5,
                    pinned: index == 0 || index == 1 || index == 2,
                    favicon_url: Some("http://www.saltybet.com/favicon.ico".to_owned()),
                    url: Some("https://www.example.com/foo?bar#qux".to_owned()),
                    title: Some(format!("Foo {}", index)),
                }
            }).collect(),
        };


    }).forget();*/
}