// ES5 only: this runs in the 2012 Opera (Presto) HMI. Use var and function expressions.
RevisedHMI.register("{{name}}/{{name}}", {
  open: function (ctx) {
    var link = document.createElement("link");
    link.rel = "stylesheet";
    link.href = ctx.assets + "style.css";
    document.getElementsByTagName("head")[0].appendChild(link);

    var title = document.createElement("div");
    title.className = "app-{{name}}-title";
    title.appendChild(document.createTextNode("{{name}}"));

    var row = document.createElement("div");
    row.className = "app-{{name}}-row app-{{name}}-row-focus";
    row.appendChild(document.createTextNode("Hello from {{name}}"));

    ctx.root.appendChild(title);
    ctx.root.appendChild(row);
    ctx.setTitle("{{name}}");
  },
  close: function () {}
});
