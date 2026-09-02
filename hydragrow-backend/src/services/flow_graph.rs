use std::collections::{HashMap, HashSet};

/// Trả về Err(mô tả chu trình) nếu thêm/sửa `candidate` (id + next_flow_ids mới) vào tập
/// `existing` (toàn bộ script CÙNG kind hoặc liên quan, đã load từ DB, KHÔNG gồm candidate cũ nếu là update)
/// sẽ tạo chu trình trong đồ thị next_flow_ids. Tự-trỏ (`candidate.id` nằm trong chính
/// `candidate.next_flow_ids`) cũng tính là chu trình.
pub fn detect_cycle(
    candidate_id: &str,
    candidate_next_flow_ids: &[String],
    existing: &[(String, Vec<String>)],
) -> Result<(), String> {
    // Build adjacency graph: id -> next_flow_ids
    let mut graph: HashMap<&str, &[String]> = HashMap::new();
    for (id, next_ids) in existing {
        graph.insert(id.as_str(), next_ids.as_slice());
    }
    graph.insert(candidate_id, candidate_next_flow_ids);

    // DFS from candidate_id to see if candidate_id can be reached again
    let mut visited = HashSet::new();
    let mut path = HashSet::new();

    fn dfs<'a>(
        curr: &'a str,
        graph: &HashMap<&'a str, &'a [String]>,
        visited: &mut HashSet<&'a str>,
        path: &mut HashSet<&'a str>,
    ) -> bool {
        visited.insert(curr);
        path.insert(curr);

        if let Some(next_ids) = graph.get(curr) {
            for next in *next_ids {
                let next_str = next.as_str();
                if path.contains(next_str) {
                    return true;
                }
                if !visited.contains(next_str) && dfs(next_str, graph, visited, path) {
                    return true;
                }
            }
        }

        path.remove(curr);
        false
    }

    if dfs(candidate_id, &graph, &mut visited, &mut path) {
        Err(format!(
            "Phát hiện chu trình (vòng lặp) trong chuỗi Flow bắt đầu từ '{}'",
            candidate_id
        ))
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn self_reference_is_a_cycle() {
        let result = detect_cycle("a", &["a".to_string()], &[]);
        assert!(result.is_err());
    }

    #[test]
    fn two_node_cycle_is_detected() {
        // a -> b, thử thêm b -> a
        let existing = vec![("a".to_string(), vec!["b".to_string()])];
        let result = detect_cycle("b", &["a".to_string()], &existing);
        assert!(result.is_err());
    }

    #[test]
    fn dag_is_allowed() {
        let existing = vec![("a".to_string(), vec!["b".to_string()])];
        let result = detect_cycle("b", &["c".to_string()], &existing);
        assert!(result.is_ok());
    }

    #[test]
    fn unrelated_next_flow_ids_do_not_interfere() {
        let existing = vec![("x".to_string(), vec!["y".to_string()])];
        let result = detect_cycle("a", &["b".to_string()], &existing);
        assert!(result.is_ok());
    }
}
