import dataLifecycleScenario from '../../../../../examples/traffic-overview/scenarios/data-lifecycle.schemra?raw';
import everythingScenario from '../../../../../examples/traffic-overview/scenarios/everything.schemra?raw';
import markerStylesScenario from '../../../../../examples/traffic-overview/scenarios/markers.schemra?raw';
import requestResponseScenario from '../../../../../examples/traffic-overview/scenarios/request-response.schemra?raw';
import serviceFlowScenario from '../../../../../examples/traffic-overview/scenarios/service-flow.schemra?raw';
import trafficOverviewVisual from '../../../../../examples/traffic-overview/visuals/traffic-overview.schemra?raw';
import { createInitialProject } from './session';

/** Creates the built-in example project from the same checked-in sources as the CLI fixture. */
export function createExampleProject() {
	return createInitialProject(trafficOverviewVisual, requestResponseScenario, [
		{
			id: 'file-everything',
			path: 'scenarios/everything.schemra',
			source: everythingScenario
		},
		{
			id: 'file-data-lifecycle',
			path: 'scenarios/data-lifecycle.schemra',
			source: dataLifecycleScenario
		},
		{
			id: 'file-markers',
			path: 'scenarios/markers.schemra',
			source: markerStylesScenario
		},
		{
			id: 'file-service-flow',
			path: 'scenarios/service-flow.schemra',
			source: serviceFlowScenario
		}
	]);
}
