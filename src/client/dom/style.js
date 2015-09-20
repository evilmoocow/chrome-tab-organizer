import * as record from "../../util/record";
import * as list from "../../util/list";
import * as ref from "../../util/ref";
import { fail } from "../../util/assert";


export const get_style_value = (style, key) =>
  style["getPropertyValue"](key);

export const set_style_value = (() => {
  const prefixes = {
    // TODO it's a bit hacky to use the prefix system for this purpose...
    //"width": ["width", "min-width", "max-width"],
    //"height": ["height", "min-height", "max-height"],
    "box-sizing": ["-moz-box-sizing", "box-sizing"], // TODO get rid of this later
    "filter": ["-webkit-filter", "filter"]
  };

  return (style, key, value, important = false) => {
    // TODO test this
    if (typeof key !== "string") {
      fail(new Error("Key must be a string: " + key));
    }

    // TODO test this
    if (value !== null && typeof value !== "string") {
      fail(new Error("Value must be null or a string: " + value));
    }

    if (value === "") {
      fail(new Error("Value cannot be \"\", use `null` instead"));
    }

    const keys = (prefixes[key]
                   ? prefixes[key]
                   : [key]);

    const old_values = list.map(keys, (key) => get_style_value(style, key));

    const new_values = list.map(keys, (key) => {
      // TODO test this
      if (value === null) {
        style["removeProperty"](key);

      } else {
        // TODO does this trigger a relayout ?
        // https://drafts.csswg.org/cssom/#dom-cssstyledeclaration-setproperty
        style["setProperty"](key, value, (important ? "important" : ""));
      }

      return get_style_value(style, key);
    });

    // TODO test this
    const every = list.all(new_values, (new_value, i) => {
      const old_value = list.get(old_values, i);
      // TODO is this correct ?
      return (new_value === old_value) &&
             (old_value !== value);
    });

    if (every) {
      fail(new Error("Invalid key or value (\"" + key + "\": \"" + value + "\")"));
    }
  };
})();


const e = document["createElement"]("style");
e["type"] = "text/css";
// TODO does this trigger a relayout ?
document["head"]["appendChild"](e);

const sheet = e["sheet"];
const cssRules = sheet["cssRules"];

export const insert_rule = (rule) => {
  // TODO does this trigger a relayout ?
  // TODO this may not work in all browsers
  // TODO sheet.addRule(s)
  const index = sheet["insertRule"](rule + " {}", cssRules["length"]);

  return cssRules[index];
};


let style_id = 0;

export const make_style = (rules) => {
  const class_name = "__style_" + (++style_id) + "__";

  make_stylesheet("." + class_name, rules);

  return {
    _type: 0,
    _name: class_name
  };
};


export const set_rules = (style, rules) => {
  record.each(rules, (key, value) => {
    ref.listen(value, (value) => {
      set_style_value(style, key, value);
    });
  });
};

export const make_stylesheet = (name, rules) => {
  const style = insert_rule(name)["style"];

  set_rules(style, rules);
};
