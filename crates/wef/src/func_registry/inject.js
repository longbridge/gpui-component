window.jsBridge = {
  __internal: {
    call(method, args) {
      var request = {
        method: method,
        args,
      };
      return new Promise((resolve, reject) => {
        window.cefQuery({
          request: JSON.stringify(request),
          persistent: false,
          onSuccess: function (response) {
            resolve(JSON.parse(response));
          },
          onFailure: (error_code, error_message) => reject(error_message),
        });
      });
    },
    nextEventListenerId: 0,
    eventListeners: {},
    emit(message) {
      for (const id in this.eventListeners) {
        this.eventListeners[id](message);
      }
    },
  },
  addEventListener(callback) {
    const id = this.__internal.nextEventListenerId++;
    this.__internal.eventListeners[id] = callback;
    return id;
  },
  removeEventListener(id) {
    delete this.__internal.eventListeners[id];
  },
};
