use crate::own_css_plan::{Node, Plan};

pub(crate) fn merge(plan: &mut Plan, roots: &[usize]) -> usize {
    let root = plan.nodes.len();
    plan.nodes.push(Node::Object(Vec::new()));
    for &source in roots {
        let mut pending = vec![(root, source)];
        while let Some((target, source)) = pending.pop() {
            let Node::Object(added) = &plan.nodes[source] else {
                continue;
            };
            let added = added.clone();
            let Node::Object(mut properties) = plan.nodes[target].clone() else {
                continue;
            };
            for mut property in added {
                if let Some(index) = properties.iter().position(|p| p.name == property.name) {
                    let previous = properties.remove(index);
                    if let (Node::Object(_), Node::Object(_)) =
                        (&plan.nodes[previous.value], &plan.nodes[property.value])
                    {
                        let id = plan.nodes.len();
                        plan.nodes.push(plan.nodes[previous.value].clone());
                        pending.push((id, property.value));
                        property.value = id;
                    }
                }
                properties.push(property);
            }
            plan.nodes[target] = Node::Object(properties);
        }
    }
    root
}
