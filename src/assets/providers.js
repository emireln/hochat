import openaiLogo from "./providers/openai.svg";
import claudeLogo from "./providers/claude.svg";
import geminiLogo from "./providers/gemini.svg";
import deepseekLogo from "./providers/deepseek.svg";
import glmLogo from "./providers/glm.svg";
import ollamaLogo from "./providers/ollama.svg";

export const providerLogos = {
  openai: openaiLogo,
  claude: claudeLogo,
  gemini: geminiLogo,
  deepseek: deepseekLogo,
  glm: glmLogo,
  ollama: ollamaLogo,
};

export function getProviderLogo(providerId) {
  return providerLogos[providerId] || null;
}
