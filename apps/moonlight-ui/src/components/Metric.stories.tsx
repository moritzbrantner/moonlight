import { Activity } from "lucide-react";
import type { Meta, StoryObj } from "@storybook/react-vite";
import { Metric } from "./Metric";

const meta = {
  title: "Components/Metric",
  component: Metric
} satisfies Meta<typeof Metric>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Default: Story = {
  args: {
    label: "Total",
    value: 42,
    icon: <Activity size={18} />
  }
};
