## Arena Inference
We do not use Garbage Collection, Reference Counting, or explicit calls to `free()`. We use **Region-Based Memory Management (Arena Inference)**.

*   **How it works:** The compiler builds a reachability graph. If `alloc(A)` points to `alloc(B)`, they are placed in the same **Arena**. 
*   **Lifetimes:** The compiler finds the longest-living member in that graph. At the start of the scope of that member, it emits `create_arena()`. At the end of that scope, it emits `free_arena()`.
*   **Performance:** Allocations are fast (bump-pointer). Deallocations are O(1). 
*   **Safety:** Because everything connected lives in the same Arena, Use-After-Free and Double-Free bugs are impossible.