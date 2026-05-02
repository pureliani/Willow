## 1. Memory Management: Arena Inference
We do not use Garbage Collection, Reference Counting, or explicit `free()`. We use **Region-Based Memory Management (Arena Inference)**.

*   **How it works:** The compiler builds a reachability graph. If `alloc(A)` points to `alloc(B)`, they are placed in the same **Arena**. 
*   **Lifetimes:** The compiler finds the longest-living member in that graph. At the start of the scope of that member, it emits `create_arena()`. At the end of that scope, it emits `free_arena()`.
*   **Performance:** Allocations are fast (bump-pointer). Deallocations are O(1). 
*   **Safety:** Because everything connected lives in the same Arena, Use-After-Free and Double-Free bugs are impossible.

## 2. The Type System, Aliasing, & Narrowing
The language uses **Invariant Structural Typing** with **Tagged Unions** (e.g., `string | i32`). We allow arbitrary mutable aliasing, but we manage the "Accidental Widening" problem using a strict set of rules.

*   **Flow-Sensitive Narrowing:** If the developer writes `if x::is(String)`, the compiler tracks that `x` is a `string` inside the `then` block.
*   **Intra-procedural (Local) Aliasing:** Arbitrary aliasing is allowed locally (`let y = x;`). The compiler's `PointsToAnalyzer` tracks this.
*   **The Mutation Rule:** If `A` and `B` alias (or *might* alias) the same memory, and `A` is mutated (`A = new_val`), the compiler checks: *"Can `new_val` be stored in `B`'s narrowed type?"*
    *   If **Yes** (e.g., assigning a string to a string): `B` keeps its narrowing.
    *   If **No** (e.g., assigning an i32 to a string): `B`'s narrowing is invalidated, and it reverts to its base union type (`string | i32`).
*   **Inter-procedural (Function Calls) - Default:** If we pass arguments to a function, the callee assumes they *might* alias. If the callee mutates one, it applies the **Mutation Rule** to all other arguments that might alias it.
*   **The `noalias` Keyword:** 
    *   *Signature:* `fn process(a: noalias<{ foo: i32 }>, b: noalias<{ foo: i32 }>)`
    *   *Caller Rule:* The caller's compiler **must prove** that `a` and `b` do not alias each other (have disjoint reachability sets). If they overlap, compilation fails.
    *   *Callee Rule:* The callee trusts the caller. It knows `a` and `b` are disjoint, so mutating `b` bypasses the Mutation Rule for `a`. `a`'s narrowed type is perfectly preserved.
    *   *Re-use:* `noalias` does **not** consume the variable. The caller can continue using the variables after the function returns. It acts as an "exclusive lock" only for the duration of the call.
*   **Safety:** Forgetting a narrowed type is 100% safe. It just forces the developer to write another `if x::is(T)` check.

## 3. The Analysis Pipeline
To make this work without forcing the developer to write lifetime annotations, the compiler uses **Implicit Function Summaries** and compiles **Bottom-Up** (Reverse Topological Order on the Call Graph).

When the compiler finishes analyzing a function, it generates a summary:
1.  **`escapes_to`:** Which arguments were stored inside other arguments? (Used by the caller to merge Arenas correctly and prevent Region Leaks).
2.  **`return_aliases`:** Does the return value alias any of the arguments?
3.  **`out_types`:** What is the exact narrowed type of each argument at the `return` instruction? (Allows the caller to keep narrowed types without redundant `if` checks).

## 4. Data Structures: Structs vs. Lists
The `PointsToAnalyzer` treats different data structures differently based on static knowledge:

*   **Structs (Field-Sensitive):** The compiler knows that `obj.b` and `obj.c` are different locations. It can track exact aliases and preserve precise type narrowing.
*   **Lists/Arrays (Index-Insensitive):** The compiler cannot know if `list[0]` and `list[7]` are the same at compile time. Therefore, the entire list is collapsed into a single `AbstractLocation`. Mutating *any* element in a list applies the Mutation Rule to *all* elements in that list. (Developers bypass this by binding elements to local variables: `let item = list[0];`).

## 5. Returned Structs
If a function returns a struct, the caller doesn't inherently know if the struct has internal aliasing (e.g., `ret.x` aliases `ret.y`). 
*   **Solution:** The callee's Function Summary includes an `internal_aliases` list. The caller imports this into its Points-To graph. This allows the caller to safely mutate `ret.x` without accidentally invalidating `ret.z`, while correctly applying the Mutation Rule to `ret.y`.