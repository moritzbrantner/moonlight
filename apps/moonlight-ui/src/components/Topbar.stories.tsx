import type { Meta, StoryObj } from "@storybook/react-vite";
import { Topbar } from "./Topbar";

const meta = {
  title: "Components/Topbar",
  component: Topbar
} satisfies Meta<typeof Topbar>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Overview: Story = {
  args: {
    page: "overview",
    onNavigate: () => undefined,
    onRefresh: () => Promise.resolve()
  }
};

export const Dashboard: Story = {
  args: {
    page: "dashboard",
    onNavigate: () => undefined,
    onRefresh: () => Promise.resolve()
  }
};
