pub struct MultiUpMirrorLink {
    original_link: String,
    id: String,
    is_project: bool,
}

impl MultiUpMirrorLink {
    pub fn new(original_link: String, slug: String) -> Self {
        let (id, is_project) = process_slug(&slug);
        Self {
            original_link,
            id,
            is_project,
        }
    }

    pub fn get_mirror_link(&self) -> String {
        format!("https://multiup.io/en/mirror/{}", self.id)
    }
}

fn process_slug(slug: &str) -> (String, bool) {
    let mut id = String::new();
    let mut is_project = false;
    for path in slug.split('/') {
        if path.len() == 32 {
            path.clone_into(&mut id);
        } else if path == "project" {
            is_project = true;
        }
    }

    return (id, is_project);
}


// Tests
#[cfg(test)]
mod tests {
    #[test]
    fn test_process_slug() {
        // Short download link
        let slug = "507cf0314f5f29d404a2425db90e0dd1";
        let (id, is_project) = super::process_slug(slug);
        assert_eq!(id, "507cf0314f5f29d404a2425db90e0dd1");
        assert!(!is_project);

        // Long download link
        let slug = "download/507cf0314f5f29d404a2425db90e0dd1/The Night Way Home v1.065.7z";
        let (id, is_project) = super::process_slug(slug);
        assert_eq!(id, "507cf0314f5f29d404a2425db90e0dd1");
        assert!(!is_project);

        // Project link
        let slug = "project/1e8a98c29c6a1db357f447ed278d0594";
        let (id, is_project) = super::process_slug(slug);
        assert_eq!(id, "1e8a98c29c6a1db357f447ed278d0594");
        assert!(is_project);
    }
}