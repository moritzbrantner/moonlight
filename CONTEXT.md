# Moonlight

Moonlight compares behavior across reference and candidate targets, captures what each target produced, and classifies the result.

## Language

**Moonlight**:
A behavior comparison system that invokes reference and candidate targets, captures their observations, and classifies the comparison.
_Avoid_: The previous accidental product name

**Comparison Run**:
One captured comparison across a required candidate target, a primary reference target, and an optional secondary reference target.
_Avoid_: Request Record

**Target**:
A service, command, or future adapter-specific executable unit invoked by Moonlight.
_Avoid_: Backend

**Primary Reference**:
The main reference target used as the baseline.

**Secondary Reference**:
An optional second reference target used to detect reference instability.

**Candidate**:
The target being evaluated against reference behavior.

**Target Observation**:
The captured output from one target during a run.
_Avoid_: Backend Capture

**Suspicious Difference**:
Candidate behavior that differs from stable reference behavior.

**Reference Noise**:
Behavior that differs between primary and secondary references.

**Target Error**:
A configured target failed to produce an observation.
