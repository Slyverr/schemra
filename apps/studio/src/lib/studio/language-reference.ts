export const languageReferenceSections = [
	{
		title: 'reference.visualsTitle',
		description: 'reference.visualsDescription',
		source: `visual request_flow "Request flow" {
  direction: left-to-right
  group services "Services" {
    layout: grid
    columns: 2
    node client "Client" {
      icon: "user"
      detail: "Sends requests"
    }
    node api "API" {
      icon: "server"
      detail: "Processes requests"
    }
  }
  client -> api
}`
	},
	{
		title: 'reference.scenariosTitle',
		description: 'reference.scenariosDescription',
		source: `scenario request_flow_demo "Request flow" for visual request_flow {
  token request {
    icon: "mail"
  }
  parallel {
    move request from client -> api {
      duration: 700ms
    }
    highlight api {
      duration: 450ms
    }
    wait 450ms
  }
}`
	}
] as const;
