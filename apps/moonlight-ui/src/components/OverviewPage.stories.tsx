import type { Meta, StoryObj } from "@storybook/react-vite";
import { OverviewPage } from "./OverviewPage";

const meta = {
  title: "Screens/OverviewPage",
  component: OverviewPage
} satisfies Meta<typeof OverviewPage>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Default: Story = {
  args: {
    onNavigate: () => undefined
  }
};
