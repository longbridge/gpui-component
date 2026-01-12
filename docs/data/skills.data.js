import { readdirSync, readFileSync } from "fs";
import { join } from "path";
import { fileURLToPath } from "url";
import { dirname } from "path";

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

function parseFrontmatter(content) {
  const frontmatterRegex = /^---\s*\n([\s\S]*?)\n---\s*\n([\s\S]*)$/;
  const match = content.match(frontmatterRegex);
  
  if (!match) {
    return { frontmatter: {}, content: content.trim() };
  }
  
  const frontmatterText = match[1];
  const body = match[2];
  
  const frontmatter = {};
  frontmatterText.split('\n').forEach(line => {
    const colonIndex = line.indexOf(':');
    if (colonIndex > 0) {
      const key = line.substring(0, colonIndex).trim();
      let value = line.substring(colonIndex + 1).trim();
      // Remove quotes if present
      if ((value.startsWith('"') && value.endsWith('"')) || 
          (value.startsWith("'") && value.endsWith("'"))) {
        value = value.slice(1, -1);
      }
      frontmatter[key] = value;
    }
  });
  
  return { frontmatter, content: body.trim() };
}

export default {
  async load() {
    const skillsDir = join(__dirname, "../../.claude/skills");
    const skills = [];
    
    try {
      const skillDirs = readdirSync(skillsDir, { withFileTypes: true })
        .filter(dirent => dirent.isDirectory())
        .map(dirent => dirent.name);
      
      for (const skillDir of skillDirs) {
        const skillPath = join(skillsDir, skillDir, "SKILL.md");
        try {
          const content = readFileSync(skillPath, "utf-8");
          const { frontmatter, content: body } = parseFrontmatter(content);
          
          skills.push({
            id: skillDir,
            name: frontmatter.name || skillDir,
            description: frontmatter.description || "",
            content: body,
            path: skillPath,
          });
        } catch (err) {
          console.warn(`Failed to read skill ${skillDir}:`, err.message);
        }
      }
      
      // Sort skills by name
      skills.sort((a, b) => a.name.localeCompare(b.name));
      
      return skills;
    } catch (err) {
      console.error("Failed to load skills:", err);
      return [];
    }
  },
};
