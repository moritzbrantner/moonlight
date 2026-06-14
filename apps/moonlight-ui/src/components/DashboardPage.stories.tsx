import type { Meta, StoryObj } from "@storybook/react-vite";
import { configFixture, runFixture, runListFixture, statsFixture } from "../test/fixtures";
import { DashboardPage } from "./DashboardPage";

const meta = {
  title: "Screens/DashboardPage",
  component: DashboardPage
} satisfies Meta<typeof DashboardPage>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Loaded: Story = {
  args: {
    config: configFixture,
    error: null,
    loading: false,
    onSelectRun: () => undefined,
    runs: runListFixture,
    selected: runFixture,
    selectedFromList: runListFixture[0],
    selectedId: runFixture.id,
    stats: statsFixture
  }
};

export const Loading: Story = {
  args: {
    ...Loaded.args,
    loading: true
  }
};

export const Error: Story = {
  args: {
    ...Loaded.args,
    error: "API failed"
  }
};

export const Empty: Story = {
  args: {
    config: null,
    error: null,
    loading: false,
    onSelectRun: () => undefined,
    runs: [],
    selected: null,
    selectedFromList: null,
    selectedId: null,
    stats: null
  }
};
