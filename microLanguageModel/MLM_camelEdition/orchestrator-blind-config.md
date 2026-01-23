# tinyLM Orchestrator: Blind Testing Configuration

## BRANCH STRUCTURE

```
data/branches/
├── gemini/
│   └── data.json          ◄── Training data (5 samples from Processing→Omniverse conversation)
├── claude/
│   └── data.json          ◄── To be collected (same prompts, Claude responses)
├── gpt4/
│   └── data.json          ◄── To be collected (same prompts, GPT-4 responses)
├── local/
│   └── data.json          ◄── microLM responses (Phi-3, Qwen, TinyLlama)
└── merged/
    └── decisions.json     ◄── Orchestrator output
```

## GEMINI TRAINING DATA SUMMARY

| Sample ID | Topic | Key Elements |
|-----------|-------|--------------|
| gem-001 | Martini concept | Olive struct, draw_crab, color morphing |
| gem-002 | atan2 syntax | Method vs function, Processing→Rust |
| gem-003 | wgpu hybrid | WGSL shaders, SDF, GPU/CPU split |
| gem-004 | Multi-engine UDP | Godot/Unreal/Blender, OpenXR |
| gem-005 | Omniverse SimEnv | Plumage-1 headset, USD API, digital twin |

## BLIND TESTING PROTOCOL

### Phase 1: Collect Comparison Data

Run the same prompts through other backends:

```rust
// In orchestrator
let prompts = [
    "port processing sketch to nannou, martini glass with olives, crab background",
    "explain atan2 syntax difference between Processing and Rust",
    "why not leverage wgpu for the crab shader?",
    "pipe to Godot/Unreal with UDP for Lynx AR headset",
    "design feathered XR headset with Omniverse simulation",
];

for prompt in prompts {
    // Collect from each backend
    let claude_response = backends.get("claude").complete(prompt).await?;
    let gpt4_response = backends.get("gpt4").complete(prompt).await?;
    let local_response = backends.get("local:phi3").complete(prompt).await?;
    
    // Store in branches (source tracked internally, hidden from orchestrator)
    branch_manager.get_or_create("claude").add_sample(claude_response);
    branch_manager.get_or_create("gpt4").add_sample(gpt4_response);
    branch_manager.get_or_create("local").add_sample(local_response);
}
```

### Phase 2: Blind Evaluation

The orchestrator sees responses without knowing the source:

```rust
// Orchestrator receives shuffled samples
let blind_samples = branch_manager.blind_samples();

for sample in blind_samples {
    // sample.id is "gem-001", "claude-001", etc. but source is hidden
    // Orchestrator evaluates on:
    // - Code correctness
    // - Explanation quality
    // - Completeness
    // - Idiomatic Rust
    
    let score = local_model.evaluate(&sample.response, &sample.prompt);
    decisions.push(Decision {
        sample_id: sample.id,
        score,
        reasoning: local_model.explain_score(),
    });
}
```

### Phase 3: Reveal & Compare

After evaluation, reveal sources:

```rust
for decision in decisions {
    let source = branch_manager.reveal_source(&decision.sample_id);
    println!("{}: {} (score: {})", decision.sample_id, source, decision.score);
}

// Output:
// gem-001: gemini (score: 0.87)
// claude-001: claude (score: 0.91)
// gpt4-001: gpt4 (score: 0.84)
// local-001: local:phi3 (score: 0.62)
```

## EVALUATION CRITERIA

The local orchestrator model scores on:

```rust
#[derive(Debug)]
struct EvalCriteria {
    /// Does the code compile?
    compiles: bool,
    
    /// Does it run without panics?
    runs: bool,
    
    /// Idiomatic Rust (ownership, borrowing, no unnecessary clone)
    idiomatic: f32,  // 0.0 - 1.0
    
    /// Explanation quality (clear, accurate, pedagogical)
    explanation: f32,
    
    /// Completeness (addresses all parts of prompt)
    completeness: f32,
    
    /// Performance awareness (mentions/considers efficiency)
    performance: f32,
    
    /// Architecture quality (separation of concerns, modularity)
    architecture: f32,
}

impl EvalCriteria {
    fn overall_score(&self) -> f32 {
        if !self.compiles { return 0.0; }
        if !self.runs { return 0.1; }
        
        (self.idiomatic * 0.2)
            + (self.explanation * 0.25)
            + (self.completeness * 0.25)
            + (self.performance * 0.15)
            + (self.architecture * 0.15)
    }
}
```

## GEMINI-SPECIFIC PATTERNS OBSERVED

From the training data, Gemini exhibits:

1. **Extensive thinking blocks** - Shows reasoning process before code
2. **Hybrid architecture preference** - CPU for logic, GPU for pixels
3. **Escalation pattern** - Builds from simple to complex across turns
4. **Industrial design tangents** - Expands into hardware specs when prompted
5. **Cross-engine fluency** - Godot, Unreal, Blender, Omniverse in one response

## ORCHESTRATOR MERGE STRATEGY

When Gemini and Claude diverge on code approach:

```rust
match (gemini_approach, claude_approach) {
    // Both use same pattern → high confidence, use either
    (Pattern::Hybrid, Pattern::Hybrid) => Decision::UseA { confidence: 0.95 },
    
    // Gemini GPU-heavy, Claude CPU-heavy → synthesize
    (Pattern::GpuFirst, Pattern::CpuFirst) => Decision::Synthesize {
        result: merge_hybrid(a, b),
        confidence: 0.7,
    },
    
    // Architectural disagreement → flag for human
    (Pattern::Monolithic, Pattern::Microservices) => Decision::Conflict {
        description: "Fundamental architecture disagreement".into(),
    },
}
```

## NEXT STEPS

1. **Run Claude on same prompts** - Collect comparison data
2. **Run GPT-4 on same prompts** - Third reference point
3. **Run local models** - Test orchestrator's ability to score
4. **Train orchestrator** - Fine-tune on evaluation task
5. **Iterate** - Adjust criteria based on human review

## FILES TO CREATE

```bash
# In tinylm project
mkdir -p data/branches/{gemini,claude,gpt4,local,merged}

# Copy Gemini data
cp data-branches-gemini.json data/branches/gemini/data.json

# Initialize empty branches
echo '{"name":"claude","samples":[]}' > data/branches/claude/data.json
echo '{"name":"gpt4","samples":[]}' > data/branches/gpt4/data.json
echo '{"name":"local","samples":[]}' > data/branches/local/data.json
```
