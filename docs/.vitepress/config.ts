import { defineConfig } from "vitepress";
import { withMermaid } from "vitepress-plugin-mermaid";

export default withMermaid(
  defineConfig({
    title: "concord",
    description: "Type-safe API bindings for Rust",
    themeConfig: {
      nav: [
        { text: "Guide", link: "/guide/" },
        { text: "API Reference", link: "/api/" },
        { text: "RHI", link: "https://rhi.zone/" },
      ],
      sidebar: {
        "/guide/": [
          {
            text: "Getting Started",
            items: [
              { text: "Introduction", link: "/guide/" },
              { text: "Installation", link: "/guide/installation" },
            ],
          },
          {
            text: "Bindings",
            items: [
              { text: "Web APIs", link: "/guide/web-apis" },
              { text: "OpenAPI", link: "/guide/openapi" },
              { text: "FFI", link: "/guide/ffi" },
            ],
          },
        ],
      },
      socialLinks: [{ icon: "github", link: "https://github.com/rhi-zone/concord" }],
    },
  })
);
