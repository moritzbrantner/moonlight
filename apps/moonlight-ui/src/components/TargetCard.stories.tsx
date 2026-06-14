import type { Meta, StoryObj } from "@storybook/react-vite";
import { runFixture, targetErrorFixture } from "../test/fixtures";
import { TargetCard } from "./TargetCard";

const meta = {
  title: "Components/TargetCard",
  component: TargetCard
} satisfies Meta<typeof TargetCard>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Success: Story = {
  args: {
    title: "Candidate",
    target: runFixture.candidate
  }
};

export const TargetError: Story = {
  args: {
    title: "Candidate",
    target: targetErrorFixture
  }
};

export const MissingSecondary: Story = {
  args: {
    title: "Secondary Reference",
    target: null
  }
};
