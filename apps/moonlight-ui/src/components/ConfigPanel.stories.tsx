import type { Meta, StoryObj } from "@storybook/react-vite";
import { configFixture } from "../test/fixtures";
import { ConfigPanel } from "./ConfigPanel";

const meta = {
  title: "Components/ConfigPanel",
  component: ConfigPanel
} satisfies Meta<typeof ConfigPanel>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Configured: Story = {
  args: {
    config: configFixture
  }
};

export const Loading: Story = {
  args: {
    config: null
  }
};
