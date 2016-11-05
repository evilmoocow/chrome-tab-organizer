"use strict";


function Events() {
  this.index = 0;
  this.length = 0;
  this.listeners = [];
}

exports.make = function () {
  return new Events();
};


function hasListeners(events) {
  return events.listeners.length !== 0;
}


function send(events, value, unit) {
  // TODO remove this later
  if (events.index !== 0) {
    throw new Error("Invalid state");
  }

  // TODO remove this later
  if (events.length !== 0) {
    throw new Error("Invalid state");
  }

  var listeners = events.listeners;

  // This causes it to not trigger listeners which are added while sending a value
  events.length = listeners.length;

  /*var length = listeners.length;

  for (var i = 0; i < length; ++i) {
    listeners[i](value)();
  }*/

  // All of this extra code is needed when a listener is removed while sending a value
  for (;;) {
    var index = events.index;

    if (index < events.length) {
      listeners[index](value)();

      ++events.index;

    } else {
      break;
    }
  }

  events.index = 0;
  events.length = 0;

  return unit;
}

exports.sendImpl = function (unit) {
  return function (value) {
    return function (events) {
      return function () {
        return send(events, value, unit);
      };
    };
  };
};


function receive(events, listener, unit) {
  events.listeners.push(listener);

  // TODO is this necessary ?
  var killed = false;

  return function () {
    if (!killed) {
      killed = true;

      // TODO make this faster ?
      var index = events.listeners.indexOf(listener);

      // TODO throw an error if it's not found ?
      if (index !== -1) {
        // TODO make this faster ?
        events.listeners.splice(index, 1);

        // This is needed when a listener is removed while sending a value
        if (index < events.length) {
          --events.length;

          // TODO test this
          if (index <= events.index) {
            --events.index;
          }
        }
      }
    }

    return unit;
  };
}

exports.receiveImpl = function (unit) {
  return function (push) {
    return function (events) {
      return function () {
        // TODO make this faster
        return receive(events, push, unit);
      };
    };
  };
};