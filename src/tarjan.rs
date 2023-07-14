// Translated from wikipedia pseudo-code
pub fn tarjan(vertices: usize, edges: &Vec<bool>) -> Vec<Vec<usize>> {
    debug_assert!(edges.len() == vertices * vertices);
    let mut connected_components = Vec::new();
    let mut component: Vec<usize> = Vec::new();
    let mut index = 0;
    // TODO: capacity?
    let mut stack = Vec::new();
    let mut indices: Vec<Option<usize>> = vec![None; vertices];
    let mut lowlink: Vec<Option<usize>> = vec![None; vertices];
    let mut onstack: Vec<bool> = vec![false; vertices];

    for v in 0..vertices {
        if indices[v].is_none() {
            strongconnect(
                v,
                vertices,
                edges,
                &mut index,
                &mut component,
                &mut connected_components,
                &mut stack,
                &mut indices,
                &mut lowlink,
                &mut onstack,
            );
        }
    }
    connected_components
}
fn strongconnect(
    v: usize,
    vertices: usize,
    edges: &Vec<bool>,
    index: &mut usize,
    component: &mut Vec<usize>,
    connected_components: &mut Vec<Vec<usize>>,
    stack: &mut Vec<usize>,
    indices: &mut Vec<Option<usize>>,
    lowlink: &mut Vec<Option<usize>>,
    onstack: &mut Vec<bool>,
) {
    indices[v] = Some(*index);
    lowlink[v] = Some(*index);
    *index += 1;
    stack.push(v);
    onstack[v] = true;

    // Consider successors of v
    // TODO: Do we want an explicit list of neighbours of w?
    for w in 0..vertices {
        if v == w || !edges[v * vertices + w] {
            continue;
        }
        if indices[w].is_none() {
            strongconnect(
                w,
                vertices,
                edges,
                index,
                component,
                connected_components,
                stack,
                indices,
                lowlink,
                onstack,
            );
            debug_assert!(lowlink[v].is_some());
            debug_assert!(lowlink[w].is_some());
            lowlink[v] = lowlink[v].zip_with(lowlink[w], |a, b| a.min(b));
        } else if onstack[w] {
            lowlink[v] = lowlink[v].zip_with(indices[w], |a, b| a.min(b));
        }
    }
    let mut w;
    if lowlink[v] == indices[v] {
        loop {
            w = stack.pop().unwrap();
            debug_assert!(onstack[w]);
            onstack[w] = false;
            component.push(w);
            if v == w {
                break;
            }
        }
        connected_components.push(component.clone());
        *component = Vec::new();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty() {
        assert_eq!(tarjan(0, &vec![]), Vec::<Vec<usize>>::new());
    }

    #[test]
    fn single() {
        let edges = vec![false];
        assert_eq!(tarjan(1, &edges), vec![vec![0]]);
    }

    #[test]
    fn two() {
        let edges = vec![false; 4];
        assert_eq!(tarjan(2, &edges), vec![vec![0], vec![1]]);
    }
}
