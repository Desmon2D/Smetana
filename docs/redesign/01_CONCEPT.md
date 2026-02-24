# Smetana Redesign: Point-First Data Model

## Motivation

The current wall-first architecture models geometry around walls as the primary primitive. Walls have centerline coordinates, thickness, two sides (left/right), T-junctions that split sides into sections, and complex offset-intersection logic for computing room inner polygons. This creates several problems:

1. **Disconnect between measurements and calculations.** Workers on-site measure wall lengths from corner to corner (inner face). The program stores centerline coordinates and derives inner dimensions through geometric offset — a theoretical computation that doesn't match real-world workflow.

2. **T-junction complexity.** When walls meet, the program must track junctions, split sides into sections, manage merge tolerances, compute miter joints, and handle edge cases (collinear walls, multi-wall hubs, force-merging endpoints). This is the largest source of bugs.

3. **Room detection fragility.** Automatic room detection via planar graph cycle enumeration works but requires careful epsilon handling, junction merging, degenerate cycle filtering, and self-intersection guards for inner polygon area computation.

4. **Sections as a workaround.** Sections were introduced to represent the real measurements a worker takes on-site, but they're derived from junction positions on the centerline — not entered directly as measurements.

## New Approach: Point-First Model

### Core Idea

**Points are the fundamental primitive.** Everything else — rooms, walls, openings — is defined as an ordered set of points.

A worker's real workflow:
1. Walk through a building, identify corners (points)
2. Measure distances between corners (edges)
3. Group corners into rooms
4. Note where walls, doors, and windows are

The new model mirrors this workflow directly.

### Data Model

#### Point

The atomic unit of geometry.

| Field | Type | Description |
|-------|------|-------------|
| id | UUID | Unique identifier |
| position | DVec2 | Canvas coordinates in mm (used for rendering) |
| height | f64 | Height at this point in mm (editable, used for calculations) |

Points are shared between objects. A corner point can simultaneously belong to multiple rooms, a wall polygon, and an opening.

#### Edge

A connection between two points with measurable properties.

| Field | Type | Description |
|-------|------|-------------|
| point_a | PointId | First endpoint |
| point_b | PointId | Second endpoint |
| distance | f64 | Length in mm. Default: computed from point coordinates. Overridable with real measurement. |
| angle | f64 | Angle in degrees relative to previous edge. Default: computed from coordinates. Overridable. |

Edges are implicitly created when two points are used together in any object (room contour, wall polygon, opening). They can also be created explicitly to record a measurement between any two points.

#### Room

An ordered set of points forming a closed contour, with optional cutouts.

| Field | Type | Description |
|-------|------|-------------|
| id | UUID | Unique identifier |
| name | String | Display name (e.g., "Kitchen") |
| points | Vec\<PointId\> | Outer contour, ordered (CW or CCW) |
| cutouts | Vec\<Vec\<PointId\>\> | Inner contours (columns, shafts, etc.) |

Computed properties:
- **Perimeter** = sum of edge distances around the outer contour
- **Floor area** = Shoelace(outer contour) - sum of Shoelace(cutouts)
- **Wall area per edge** = edge.distance x avg(point_a.height, point_b.height) - opening areas on that edge

#### Wall (Visual)

A polygon filled with a wall color on the canvas. Purely visual — does not participate in calculations.

| Field | Type | Description |
|-------|------|-------------|
| id | UUID | Unique identifier |
| points | Vec\<PointId\> | Polygon vertices (typically 4 for a simple wall segment) |
| color | Color | Fill color |

Walls serve only for visual representation on the floor plan. Calculation-relevant wall data (length, height, area) comes from room edges.

#### Opening

A polygon representing a door, window, or other opening.

| Field | Type | Description |
|-------|------|-------------|
| id | UUID | Unique identifier |
| points | Vec\<PointId\> | Polygon vertices (4+ points) |
| kind | OpeningKind | Door or Window |
| height | f64 | Opening height in mm |
| width | f64 | Opening width in mm (can be derived from polygon) |
| sill_height | f64 | Window sill height (windows only) |
| reveal_width | f64 | Window reveal width (windows only) |

Opening association with room edges is determined by shared points: if 2+ opening points lie on a room edge, the opening belongs to that edge.

### What Gets Removed

| Current Component | Why It's Removed |
|-------------------|-----------------|
| SideData, SideJunction, SectionData | Replaced by edges between points |
| room_metrics.rs (offset-intersect) | Replaced by Shoelace on edge distances + angles |
| room_detection.rs (cycle detection) | Rooms are created manually by selecting points |
| wall_joints.rs, endpoint_merge.rs | No joints needed — walls are simple filled polygons |
| T-junction snap logic | Simplified to point snap + grid snap |

### User Workflow

#### Creating geometry
1. Select the Point tool, click on the canvas to place points at room corners
2. Points snap to grid and to existing points

#### Creating a room
1. Select the Room tool
2. Click points in order around the room contour
3. The room polygon appears with fill color
4. Optionally add cutouts (columns) by selecting additional point sequences

#### Creating a wall (visual)
1. Select the Wall tool
2. Click points defining the wall polygon (e.g., 4 points for a rectangular wall segment)
3. The polygon fills with the wall color

#### Creating an opening
1. Select the Door or Window tool
2. Click 4+ points defining the opening footprint
3. Set parameters (height, sill height, etc.) in the properties panel

#### Editing measurements
1. Select an edge (click on a line between two points)
2. In the properties panel, override distance and/or angle with real measurements
3. Room area and perimeter update automatically

### Canvas Interaction

#### Hit-testing priority (top to bottom)
1. **Points** — always clickable, ~8-10px radius, rendered on top of everything
2. **Edges** — lines between points, ~5px click zone
3. **Openings** — opening polygons
4. **Walls** — wall fill polygons
5. **Rooms** — room fill polygons
6. **Empty** — deselect

#### Visibility modes
| Mode | Visible | Use case |
|------|---------|----------|
| All | Points + edges + walls + rooms + openings | Overview |
| Wireframe | Points + edges only | Geometry editing |
| Rooms | Points + rooms (no walls) | Room work |

### Handling Concave Rooms

Concave (L-shaped, U-shaped) rooms work without special handling:
- User places points along the contour, including at reflex angles
- Shoelace formula correctly computes area for any simple (non-self-intersecting) polygon
- Canvas rendering uses earcutr triangulation (already in the project) for correct fill

### Handling Cutouts (Columns, Shafts)

A cutout is an inner contour subtracted from the room:

```
  +------------------+
  |                  |
  |    +------+      |    cutout = 4 points (column)
  |    | //// |      |
  |    +------+      |
  |                  |
  +------------------+
      room = 4 points
```

- Floor area = area(outer contour) - sum(area(cutout))
- Perimeter: outer contour only (cutout perimeter tracked separately if needed for services)
- Rendering: earcutr with hole indices for correct visual fill
