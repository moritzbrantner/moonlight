import type { Preview } from "@storybook/react-vite";
import "../src/styles/index.css";

const preview: Preview = {
  parameters: {
    a11y: {
      options: {
        runOnly: {
          type: "tag",
          values: ["wcag2a", "wcag2aa", "wcag21a", "wcag21aa"]
        }
      }
    },
    backgrounds: {
      default: "Moonlight",
      values: [
        { name: "Moonlight", value: "#f6f7fb" },
        { name: "Dark", value: "#111827" }
      ]
    }
  }
};

export default preview;
