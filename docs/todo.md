Floor Plan Editor — Requirements

1. Canvas Labels: Section Dimensions and Wall Thickness
Each wall side section must display {length} - {area} (e.g. 3500 mm - 9.45 m^2) instead of area alone. The label must be drawn parallel to the wall centerline, centered on the section. Additionally, wall thickness must be rendered as a label at the center of the wall body, also parallel to the wall, replacing the current baseline-length label.

2. Snap System Fixes and Junction Creation Rules
- Junction creation asymmetry: A T-junction is only created when the end point of a new wall lands on the side edge of an existing wall. Starting a wall from a wall-edge snap must only attach the start vertex without splitting the host side. Apply this logic consistently in the wall-creation handler.
- Phantom wall on grid coincidence: When confirming the second point of a wall, vertex snap must always take priority over grid snap. Increase the vertex snap radius or restructure snap priority in snap() so vertex snap beats grid snap unconditionally.
- Snap indicator: Render a visible on-canvas indicator (colored ring or crosshair) at the current snapped position during wall drawing, color-coded by SnapType (e.g. green = Vertex, yellow = WallEdge, white = Grid).

3. Room Validity: Require Closed Contours
Enforce that a room only exists if its wall list forms a fully closed contour. Rooms that become open due to wall deletion or modification must be immediately removed from Project.rooms.

4. Room Area Computation from Section Lengths
Replace the current area computation with a section-length-based approach:
- Interior length of each room boundary segment is taken from SideData.sections of the room-facing side. Section lengths already represent interior measurements (wall thickness is accounted for by junctions).
- Approximate each interior corner angle and add/subtract a corner area correction term. This accounts for real-world imprecision.
- Column-wall exception: When both sides of a wall face the same room interior (wall acting as a column), include its depth (thickness) in the calculation by subtracting the column cross-section area from the room area.
- Expose two computed values per room (see requirement 7).

5. Scrollbars for Side Panels
Wrap the contents of the properties panel and the project structure panel in egui::ScrollArea::vertical(). Both panels can overflow vertically when many items, sections, or services are listed.

6. Section Length Editing; Side Length Locking
- Each SectionData.length must be individually editable via a numeric input in the properties panel.
- When a side has junctions, its total length field must become read-only and computed as: sum of all section lengths + thicknesses of all walls creating junctions on that side.
- When a side has no junctions, the length field remains manually editable as before.

7. Room Properties: Two Area Values

The properties panel for a selected room must display:
- Gross area — including wall volume and window reveals (full bounding polygon).
- Net area — clear interior floor area, from the section-length-based polygon.

Both in m² with two decimal places.

8. Wall Endpoint Handles: Selection-Gated Rendering
Green and yellow endpoint handles must only render when the wall is currently selected. Do not draw them for unselected walls.

9. Wall Side Coloring and Selection Highlight
- Every SideData must always have at least one implicit section (minimum 1). When no junctions exist, treat it as one section spanning the full side length.
- Render sections with distinct colors per side (e.g. left = light blue tint, right = light orange tint) so sides are always visually distinguishable.
- Selection highlight fix: Instead of flood-filling sections blue on selection, draw a selection outline stroke around the entire wall polygon. Section fill colors remain visible underneath, preserving side identification.
- Section {length} - {area} labels (requirement 1) must appear for all sections including the implicit single-section case.