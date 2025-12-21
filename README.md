# Minuet: A Holographic Database

> "A mind graft, not a translation layer."

## Project Vision
Minuet is a holographic database built on amari-fusion's tropical-dual-Clifford algebra. Named after Star Trek's first sentient hologram, Minuet provides memory that participates in cognition rather than merely serving it.
The core proposition: retrieval should be a native algebraic operation, not index lookup with a translation layer. Queries are pattern completions in the same representational space as the stored knowledge.
What Minuet Is

- compositional associative memory where relationships are first-class
- system where "find X related to Y as A is related to B" is a single operation
- database that degrades gracefully under noise, partial queries, and capacity pressure
- substrate for reasoning, not just storage

## What Minuet Is Not

- replacement for vector databases at scale (capacity is O(dim / log dim), ~hundreds to low thousands of items per memory)
- general-purpose DBMS
- n-embedding similarity search engine

## Intended Use Cases
Minuet targets domains where relational/compositional structure matters more than scale:

Domain | Key Operation
--- | ---
Drug discovery | Molecular analogy: "X relates to target T as drug D relates to its target"
Robotics | Motor primitive composition with native SE(3) geometryCode understandingSemantic search and refactoring-as-transformation
Music/Audio | Timbral relationships, harmonic structure, gestural vocabularies
Legal reasoning | Precedent retrieval by analogical structure
Education | Concept prerequisites, misconception repair, transfer learning
Multi-agent systems | Mergeable world models, theory of mind
Neurosymbolic AI | Symbol grounding with compositional generalization
Scientific instrumentation | Anomaly characterization, experimental design retrieval


