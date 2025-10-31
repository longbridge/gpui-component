const IGNORE_LOGINS = ["dependabot[bot]", "copilot"];
const API_URL =
  "https://api.github.com/repos/longbridge/gpui-component/contributors";

export default {
  async load() {
    return await fetch(API_URL)
      .then((res) => res.json())
      .then((items) => {
        let filtered = items.filter(
          (item) => !IGNORE_LOGINS.includes(item.login.toLowerCase()),
        );
        return filtered.slice(0, 24);
      });
  },
};
