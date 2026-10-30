# Example project

The example project uses one visual with several scenarios so the full language
surface can be explored without switching diagrams.

| Source | What it demonstrates |
| --- | --- |
| `traffic-overview/visuals/traffic-overview.schemra` | Repeated node and link declarations, groups, flow direction, grid and automatic group layouts, node labels, icons, details, and both directed and undirected links. |
| `traffic-overview/scenarios/everything.schemra` | Tokens, marker icons, movement durations, parallel moves, waits, repeats, default and per-move highlights, disabled highlights, and standalone highlights. |
| `traffic-overview/scenarios/request-response.schemra` | A request and response through the service graph. |
| `traffic-overview/scenarios/service-flow.schemra` | Parallel client requests followed by a service and response flow. |
| `traffic-overview/scenarios/data-lifecycle.schemra` | Data movement, a wait, and a database highlight. |
| `traffic-overview/scenarios/markers.schemra` | Different marker icons, parallel arrivals, highlight defaults and overrides. |

Keep this example project to one visual. Add or update scenarios when language
features change, and keep the example project contract test passing.
