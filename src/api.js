import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

export const api = {
  listChats: () => invoke("list_chats"),
  createChat: () => invoke("create_chat"),
  renameChat: (chatId, title) => invoke("rename_chat", { chatId, title }),
  deleteChat: (chatId) => invoke("delete_chat", { chatId }),
  listMessages: (chatId) => invoke("list_messages", { chatId }),
  sendMessage: (chatId, content, useRag) =>
    invoke("send_message", { chatId, content, useRag }),
  stopGeneration: (chatId) => invoke("stop_generation", { chatId }),

  listModels: (provider) => invoke("list_models", { provider }),
  checkProvider: (provider) => invoke("check_provider", { provider }),
  getSettings: () => invoke("get_settings"),
  saveSettings: (patch) => invoke("save_settings", { patch }),

  ragStatus: () => invoke("rag_status"),
  listDocuments: () => invoke("list_documents"),
  deleteDocument: (documentId) => invoke("delete_document", { documentId }),
  ingestFile: (path) => invoke("ingest_file", { path }),

  storageInfo: () => invoke("storage_info"),
  openDataFolder: () => invoke("open_data_folder"),
  clearAllChats: () => invoke("clear_all_chats"),
};

export { listen };
