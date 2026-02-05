use std::collections::HashSet;

use org_eclipse_elk_core::org::eclipse::elk::core::math::KVector;

use crate::org::eclipse::elk::alg::common::t_edge::TEdge;
use crate::org::eclipse::elk::alg::common::t_triangle::TTriangle;
use crate::org::eclipse::elk::alg::common::utils::SVGImage;

pub struct BowyerWatsonTriangulation;

impl BowyerWatsonTriangulation {
    pub fn triangulate(vertices: &[KVector], debug_output_file: Option<&str>) -> HashSet<TEdge> {
        let mut svg = SVGImage::new(debug_output_file);
        svg.add_groups(&["invalid", "tri", "bndry", "done", "new"]);

        let mut topleft = KVector::with_values(f64::INFINITY, f64::INFINITY);
        let mut bottomright = KVector::with_values(f64::NEG_INFINITY, f64::NEG_INFINITY);
        for v in vertices {
            topleft.x = topleft.x.min(v.x);
            topleft.y = topleft.y.min(v.y);
            bottomright.x = bottomright.x.max(v.x);
            bottomright.y = bottomright.y.max(v.y);
            svg.g("bb").add_circle_with_attrs(
                v.x,
                v.y,
                18.0,
                "stroke=\"black\" stroke-width=\"1\" fill=\"lightgray\"",
            );
        }
        let size = KVector::with_values(bottomright.x - topleft.x, bottomright.y - topleft.y);
        svg.g("bb").add_rect_with_values(
            topleft.x,
            topleft.y,
            size.x,
            size.y,
            "stroke=\"blue\" stroke-width=\"4\" fill=\"none\"",
        );

        let wiggleroom = 50.0;
        let sa = KVector::with_values(topleft.x - wiggleroom, topleft.y - size.x - wiggleroom);
        let sb = KVector::with_values(topleft.x - wiggleroom, bottomright.y + size.x + wiggleroom);
        let sc = KVector::with_values(
            bottomright.x + size.y / 2.0 + wiggleroom,
            topleft.y + size.y / 2.0,
        );
        svg.g("bb").add_poly(
            "stroke=\"gray\" stroke-width=\"4\" fill=\"none\" stroke-dasharray=\"20,20\"",
            &[sa, sb, sc, sa],
        );
        let super_triangle = TTriangle::new(sa, sb, sc);
        svg.set_view_box(sa.x, sa.y, sc.x - sa.x, sb.y - sa.y);
        svg.isave();
        svg.remove_group("bb");

        let mut triangulation: HashSet<TTriangle> = HashSet::new();
        let mut invalid_triangles: Vec<TTriangle> = Vec::new();
        let mut boundary: Vec<TEdge> = Vec::new();
        triangulation.insert(super_triangle.clone());

        for vertex in vertices {
            svg.g("done").add_circle_with_attrs(
                vertex.x,
                vertex.y,
                18.0,
                "stroke=\"black\" stroke-width=\"1\" fill=\"lightgray\"",
            );
            svg.g("new").add_circle_with_attrs(
                vertex.x,
                vertex.y,
                18.0,
                "stroke=\"black\" stroke-width=\"1\" fill=\"black\"",
            );

            invalid_triangles.clear();
            for triangle in &triangulation {
                svg.g("tri").add_poly(
                    "stroke=\"black\" fill=\"none\" stroke-width=\"4\"",
                    &[triangle.a, triangle.b, triangle.c, triangle.a],
                );
                let c = triangle.get_circumcenter();
                svg.g("invalid").add_circle_with_attrs(
                    c.x,
                    c.y,
                    c.distance(&triangle.a),
                    "stroke=\"orange\" stroke-width=\"4\" fill=\"none\"",
                );
                if triangle.in_circumcircle(vertex) {
                    invalid_triangles.push(triangle.clone());
                    svg.g("invalid").add_poly(
                        "stroke=\"none\" fill=\"red\" opacity=\"0.18\"",
                        &[triangle.a, triangle.b, triangle.c, triangle.a],
                    );
                }
            }
            svg.isave();
            svg.clear_group("invalid");

            boundary.clear();
            for triangle in &invalid_triangles {
                for edge in &triangle.t_edges {
                    let mut on_boundary = true;
                    for other in &invalid_triangles {
                        if other != triangle && other.contains_edge(edge) {
                            on_boundary = false;
                            break;
                        }
                    }
                    if on_boundary {
                        boundary.push(edge.clone());
                        svg.g("bndry").add_line_with_attrs(
                            edge.u.x,
                            edge.u.y,
                            edge.v.x,
                            edge.v.y,
                            "stroke=\"purple\" stroke-width=\"18\" stroke-dasharray=\"20,20\"",
                        );
                    }
                }
            }
            svg.isave();

            for triangle in &invalid_triangles {
                triangulation.remove(triangle);
            }
            svg.clear_group("tri");
            for triangle in &triangulation {
                svg.g("tri").add_poly(
                    "stroke=\"black\" fill=\"none\" stroke-width=\"4\"",
                    &[triangle.a, triangle.b, triangle.c, triangle.a],
                );
            }
            svg.isave();

            for edge in &boundary {
                triangulation.insert(TTriangle::new(*vertex, edge.u, edge.v));
                svg.g("tri").add_poly(
                    "stroke=\"black\" fill=\"none\" stroke-width=\"4\"",
                    &[*vertex, edge.u, edge.v, *vertex],
                );
            }
            svg.isave();
            svg.clear_group("new");
            svg.clear_group("bndry");
            svg.clear_group("tri");
        }

        let mut t_edges: HashSet<TEdge> = HashSet::new();
        for triangle in &triangulation {
            for edge in &triangle.t_edges {
                t_edges.insert(edge.clone());
            }
        }

        t_edges.retain(|edge| {
            !super_triangle.contains_vertex(&edge.u) && !super_triangle.contains_vertex(&edge.v)
        });

        for edge in &t_edges {
            svg.add_line_with_attrs(
                edge.u.x,
                edge.u.y,
                edge.v.x,
                edge.v.y,
                "stroke=\"black\" stroke-width=\"4\"",
            );
        }
        svg.isave();

        t_edges
    }

    pub fn triangulate_without_debug(vertices: &[KVector]) -> HashSet<TEdge> {
        Self::triangulate(vertices, None)
    }
}
