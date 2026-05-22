// Apply the saved/system theme synchronously to avoid a flash before hydration.
(function () {
  try {
    var t = localStorage.getItem("showcase-theme");
    if (!t) t = window.matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light";
    document.documentElement.dataset.theme = t;
  } catch (e) {}
})();
