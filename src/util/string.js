export const uppercase = (s) =>
  s["toLocaleUpperCase"]();

export const lowercase = (s) =>
  s["toLocaleLowerCase"]();

export const replace = (s, re, x) =>
  s["replace"](re, x);

export const match = (s, re) =>
  re["exec"](s);

export const test = (s, re) =>
  re["test"](s);

export const split = (s, re) =>
  s["split"](re);

export const plural = (x, s) => {
  if (x === 1) {
    return x + s;
  } else {
    return x + s + "s";
  }
};

// TODO test this
// TODO better implementation, with error checking
export const slice = (x, from, to) =>
  x["slice"](from, to);

export const sort = (x, y) =>
  x["localeCompare"](y);
