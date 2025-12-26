import React, { useEffect, useRef } from 'react';
import * as d3 from 'd3';

const ConvoyProgress = ({ drones, waypoints }) => {
  const svgRef = useRef(null);

  useEffect(() => {
    if (!drones || !waypoints || !svgRef.current) return;

    // Clear previous content
    d3.select(svgRef.current).selectAll("*").remove();

    const width = 800;
    const height = 160;
    const margin = { top: 20, right: 30, bottom: 40, left: 40 };

    const svg = d3.select(svgRef.current)
      .attr("width", width)
      .attr("height", height)
      .style("background", "transparent");

    // Create scales
    const xScale = d3.scaleLinear()
      .domain([0, waypoints.length - 1])
      .range([margin.left, width - margin.right]);

    const yScale = d3.scaleLinear()
      .domain([0, 1])
      .range([height - margin.bottom, margin.top]);

    // Draw waypoint lines
    waypoints.forEach((wp, i) => {
      svg.append("line")
        .attr("x1", xScale(i))
        .attr("x2", xScale(i))
        .attr("y1", margin.top)
        .attr("y2", height - margin.bottom)
        .attr("stroke", "#333")
        .attr("stroke-width", 1)
        .attr("stroke-dasharray", "2,2");

      // Waypoint labels
      svg.append("text")
        .attr("x", xScale(i))
        .attr("y", height - 5)
        .attr("text-anchor", "middle")
        .attr("font-size", "10px")
        .attr("fill", "#666")
        .text(`WP${i + 1}`);
    });

    // Draw drone progress
    drones.forEach((drone, index) => {
      if (drone.status === 'offline') return;

      const droneX = xScale(drone.currentWaypoint + drone.progress);
      const droneY = margin.top + (index * ((height - margin.top - margin.bottom) / drones.length));

      // Progress line
      svg.append("line")
        .attr("x1", xScale(0))
        .attr("x2", droneX)
        .attr("y1", droneY)
        .attr("y2", droneY)
        .attr("stroke", drone.status === 'online' ? "#00ff00" : "#ffaa00")
        .attr("stroke-width", 2)
        .attr("opacity", 0.6);

      // Drone marker
      svg.append("circle")
        .attr("cx", droneX)
        .attr("cy", droneY)
        .attr("r", 4)
        .attr("fill", drone.status === 'online' ? "#00ff00" : "#ffaa00")
        .attr("stroke", "#fff")
        .attr("stroke-width", 1);

      // Drone label
      svg.append("text")
        .attr("x", droneX + 8)
        .attr("y", droneY + 3)
        .attr("font-size", "9px")
        .attr("fill", "#888")
        .text(drone.id);
    });

    // Title
    svg.append("text")
      .attr("x", width / 2)
      .attr("y", 12)
      .attr("text-anchor", "middle")
      .attr("font-size", "12px")
      .attr("fill", "#00ff00")
      .attr("font-weight", "bold")
      .text("CONVOY PROGRESS TRACKER");

  }, [drones, waypoints]);

  return (
    <div className="convoy-progress-container">
      <svg ref={svgRef}></svg>
    </div>
  );
};

export default ConvoyProgress;