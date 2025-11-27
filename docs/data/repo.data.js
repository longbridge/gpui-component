const API_URL = "https://api.github.com/repos/longbridge/gpui-component";

export default {
  async load() {
    return await fetch(API_URL).then((res) => {
      if (!res.ok) {
        throw new Error(`HTTP error! status: ${res.status}`);
      }
      return res.json();
    });
  },
};
