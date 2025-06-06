---
title: Some Template
date: 2024-10-06T00:00:00Z
---

# Markdown Basics

## Headers

### More Headers

#### Yet another Headers

##### Yet another Headers

> Some quote
> Some quote
> Some quote

---

> [!NOTE] Hello here is a Note

- Tip 1:
  - Tip 1:
    - Tip 1:
      - Tip 1:
        - Tip 1:
- Tip 2:
- Tip 3:

---

```rust
// Code block
fn main() {
  println!("Hello world")
}
```

```javascript
const nums = [1, 2, 3];
const sum = nums.reduce((acc, val) => acc + val, 0);
console.log(`Sum: ${sum}`);
```

```python
# Python
nums = [1, 2, 3]
sum = sum(nums)
print(f"Sum: {sum}")
```

```c
#include <stdio.h>
#include <stdlib.h>

// Structure to represent a graph edge
typedef struct {
    int src, dest, weight;
} Edge;

// Structure to represent a graph
typedef struct {
    int V, E; // Number of vertices and edges
    Edge* edges;
} Graph;

// Structure to represent a subset for union-find
typedef struct {
    int parent;
    int rank;
} Subset;

// Function to create a graph with V vertices and E edges
Graph* createGraph(int V, int E) {
    Graph* graph = (Graph*)malloc(sizeof(Graph));
    graph->V = V;
    graph->E = E;
    graph->edges = (Edge*)malloc(E * sizeof(Edge));
    return graph;
}

// Function to find the subset of an element
int find(Subset* subsets, int i) {
    if (subsets[i].parent != i) {
        subsets[i].parent = find(subsets, subsets[i].parent); // Path compression
    }
    return subsets[i].parent;
}

// Function to do union of two subsets
void unionSubsets(Subset* subsets, int x, int y) {
    int xroot = find(subsets, x);
    int yroot = find(subsets, y);

    // Union by rank
    if (subsets[xroot].rank < subsets[yroot].rank) {
        subsets[xroot].parent = yroot;
    } else if (subsets[xroot].rank > subsets[yroot].rank) {
        subsets[yroot].parent = xroot;
    } else {
        subsets[yroot].parent = xroot;
        subsets[xroot].rank++;
    }
}

// Comparison function for sorting edges
int compareEdges(const void* a, const void* b) {
    return ((Edge*)a)->weight > ((Edge*)b)->weight;
}

// Function to perform Kruskal's algorithm
void kruskalMST(Graph* graph) {
    int V = graph->V;
    Edge result[V]; // Store the resultant MST
    int e = 0;      // Index variable for result
    int i = 0;      // Index variable for sorted edges

    // Step 1: Sort all the edges in non-decreasing order of their weight
    qsort(graph->edges, graph->E, sizeof(graph->edges[0]), compareEdges);

    // Allocate memory for creating V subsets
    Subset* subsets = (Subset*)malloc(V * sizeof(Subset));

    // Initialize subsets
    for (int v = 0; v < V; ++v) {
        subsets[v].parent = v;
        subsets[v].rank = 0;
    }

    // Step 2: Iterate over sorted edges and add them to the result
    while (e < V - 1 && i < graph->E) {
        Edge nextEdge = graph->edges[i++];

        int x = find(subsets, nextEdge.src);
        int y = find(subsets, nextEdge.dest);

        // If including this edge does not cause a cycle
        if (x != y) {
            result[e++] = nextEdge; // Include it in the result
            unionSubsets(subsets, x, y); // Union of the subsets
        }
    }

    // Print the resultant MST
    printf("Edges in the Minimum Spanning Tree:\n");
    for (i = 0; i < e; ++i) {
        printf("%d -- %d == %d\n", result[i].src, result[i].dest, result[i].weight);
    }

    // Free allocated memory
    free(subsets);
    free(graph->edges);
    free(graph);
}

// Driver program to test above functions
int main() {
    /* Example graph */
    int V = 4; // Number of vertices
    int E = 5; // Number of edges
    Graph* graph = createGraph(V, E);

    // Adding edges (source, destination, weight)
    graph->edges[0] = (Edge){0, 1, 10};
    graph->edges[1] = (Edge){0, 2, 6};
    graph->edges[2] = (Edge){0, 3, 5};
    graph->edges[3] = (Edge){1, 3, 15};
    graph->edges[4] = (Edge){2, 3, 4};

    // Function call
    kruskalMST(graph);

    return 0;
}
```

```cpp
// C++
#include <iostream>
#include <vector>
#include <numeric>

int main() {
    std::vector<int> nums = {1, 2, 3};
    int sum = std::accumulate(nums.begin(), nums.end(), 0);
    std::cout << "Sum: " << sum << std::endl;
    return 0;
}
```

```go
// Go
package main

import "fmt"

func main() {
    nums := []int{1, 2, 3}
    sum := 0
    for _, num := range nums {
        sum += num
    }
    fmt.Println("Sum:", sum)
}
```

```html
<!-- HTML -->
<!doctype html>
<html lang="en">
  <head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <title>Sum Example</title>
  </head>
  <body>
    <script>
      const nums = [1, 2, 3];
      const sum = nums.reduce((acc, val) => acc + val, 0);
      document.write(`Sum: ${sum}`);
    </script>
  </body>
</html>
```

```lua
-- Lua
local nums = {1, 2, 3}
local sum = 0

for _, num in ipairs(nums) do
    sum = sum + num
end

print("Sum:", sum)
```

```css
/* CSS */
body {
  font-family: Arial, sans-serif;
}

.sum {
  color: blue;
  font-weight: bold;
}
```

```bash
# Bash
nums=(1 2 3)
sum=0

for num in "${nums[@]}"; do
    sum=$((sum + num))
done

echo "Sum: $sum"
```

`println!` is a `macro` in rust to write to the `STDOUT`

- List
  - Item 1
  - Item 2
    - Item 2.1
    - Item 2.2
  - Item 3
