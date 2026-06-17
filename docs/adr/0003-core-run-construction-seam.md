# Core run construction seam

Moonlight's core owns construction of complete `ComparisonRun` records from adapter-supplied metadata and captured targets, while CLI, HTTP, and project-eval adapters continue to own target invocation, capture timing, response selection, and persistence decisions. We chose this seam because run assembly and classification were duplicated across adapters, but a generic target-invocation interface would be premature: command execution, HTTP forwarding, and project checks do not yet share a useful invocation contract beyond producing captured targets.
