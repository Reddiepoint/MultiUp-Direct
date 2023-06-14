//pub const HELP_MESSAGE: &str = "Instructions:\n\
//            1. Paste your MultiUp links into the first box.\n\
//            You can paste multiple links separated by a new line.\n\n\
//            2. Choose whether you want MultiUp to check the validity of the hosts by checking \
//            the \"Re-check host status\" checkbox.\n\
//            If you choose this option, the validity of the links will be more up-to-date, and \
//            you will receive information about the files.\n\n\
//            3. Click \"Generate links\" to generate the direct links.\n\n\
//            4. The direct links will be displayed underneath.\n\
//            You can select a combination of links by holding down CTRL and clicking on links, \
//            select consecutive links by clicking and holding SHIFT, \
//            or mix and match!\n\n\
//            5. You can right-click on a link or selection of links to see more options, such as \
//            being able to copy the links or open the links in your browser.\n\n\
//            6. You can also use the filter menu to get the links you want.\n\
//            The \"Unknown\" checkbox includes links reported as unknown by MultiUp. \
//            The \"Unchecked\" checkbox includes links that are unable to be checked. (Links will \
//            only appear here when \"Re-check host status\" is disabled). \
//            You can also toggle which hosts you want links for. \
//            You can quickly select one host only by right clicking the host and \
//            pressing the button.\n\n\
//            Note:  Enabling \"Re-check host status\" will take longer to generate the links \
//            than disabling the feature (> 20 seconds), but it will give you more \
//            accurate information. \
//            If you get unchecked links, it may be worth it to \
//            generate the links with \"Re-check host status\" enabled.";

pub const HELP_MESSAGE: &str = "Instructions:\n\
1. Paste your MultiUp links into the first box.\n\
You can paste as many links as you want, separated by a new line. \
As long as a line starts with a link, it will be recognised. \
This means you can paste a page with links (e.g., FitGirl). \
Duplicate links will also be filtered out.\n\n\
2. You can check the \"Re-check host status\" box if you want MultiUp to verify the availability of the hosts.\n\
This will give you more information about the files, which will appear in a section called \"Link Information\".\n\
Note that enabling this feature will cause the generation times to be much longer.\n\n\
3. Click on \"Generate links\" to get the direct links.\n\
You can see the progress of the generation in the status below the box.\n\n\
4. Select the links you want to use. You can do this by:\n\t\
- Holding down CTRL to select individual links.\n\t\
- Clicking and holding SHIFT to select a range of links.\n\n\
5. Right-click on a link or selection of links to see more options, such as copying or opening the links in your browser.\n\n\
6. Use the filter menu to narrow down your choices:\n\t\
- \"Unknown\": These are the links that MultiUp could not check after verification.\n\t\
- \"Unchecked\": These are the links that were not verified by MultiUp. (Links will only appear here if you do not check the \"Re-check host status\" box).\n\t\
- Hosts: You can choose which hosts you want to see links for. You can right-click on a host and select \"Select ____ links only\" to quickly filter out the rest.";

pub(crate) const VERSION: &str = env!("CARGO_PKG_VERSION");