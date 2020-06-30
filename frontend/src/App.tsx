import React, { useEffect, useState } from "react";
import { Sankey } from "react-vis";
import axios from "axios";
import "./App.css";

interface Node {
	name: string;
	color: string;
}

interface Link {
	source: number;
	target: number;
	value: number;
	color: string;
}

interface Graph {
	nodes: Node[];
	links: Link[];
}

function App() {
	const [width, setWidth] = useState(window.innerWidth);
	const [height, setHeight] = useState(window.innerHeight);
	const [graph, setGraph] = useState({} as Graph);

	const onResize = () => {
		setWidth(window.innerWidth);
		setHeight(window.innerHeight);
	};

	useEffect(() => {
		window.addEventListener("resize", onResize, false);

		const interval = setInterval(async () => {
			let data: string = await axios.get("http://127.0.0.1:3001/jobs");
			let graph: Graph = JSON.parse(data);
			setGraph(graph);
		}, 1000);

		return () => clearInterval(interval);
	}, []);

	return (
		<div className="App">
			<Sankey
				nodes={graph.nodes}
				links={graph.links}
				width={width}
				height={height}
			/>
		</div>
	);
}

export default App;
