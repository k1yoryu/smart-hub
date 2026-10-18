import { invoke } from '@tauri-apps/api/core';
import { getCurrentWindow, LogicalSize } from '@tauri-apps/api/window';

const appWindow = getCurrentWindow();

interface ClipboardItem {
  id: string;
  type: string;
  content?: string;
  filePath?: string;
  isPinned: boolean;
  createdAt: string;
}

async function fetchHistory() {
  try {
    const history: ClipboardItem[] = await invoke('get_clipboard_history');
    const listElement = document.getElementById('history-list');
    if (!listElement) return;

    if (history.length === 0) {
      listElement.innerHTML = `<li class="clipboard-item" style="color: #a1a1aa;">История пуста. Скопируйте что-нибудь (Ctrl + C)...</li>`;
      return;
    }

    listElement.innerHTML = history
      .map(item => {
        const textContent = item.content ? escapeHtml(item.content) : '[Медиа/Файл]';
        const pinnedClass = item.isPinned ? 'pinned' : '';
        const pinIcon = '📌';

        return `
          <li class="clipboard-item ${pinnedClass}">
            <div class="content-text">${textContent}</div>
            <div class="actions">
              <button class="action-btn" title="Закрепить" onclick="window.togglePin('${item.id}')">${pinIcon}</button>
              <button class="action-btn" title="Удалить" onclick="window.deleteItem('${item.id}')">❌</button>
            </div>
          </li>
        `;
      })
      .join('');

  } catch (error) {
    console.error("Ошибка при получении истории:", error);
  }
}

(window as any).togglePin = async (id: String) => {
  await invoke('toggle_pin', { id });
  fetchHistory();
};

(window as any).deleteItem = async (id: String) => {
  await invoke('delete_item', { id });
  fetchHistory();
};

// Сворачивание: уменьшаем физический размер окна до 10px высоты
(window as any).collapseHub = async () => {
  const hub = document.getElementById('hub-container');
  if (hub) {
    hub.classList.remove('expanded');
  }
  // Уменьшаем физическое окно до 10px, чтобы весь экран снизу был свободен
  await appWindow.setSize(new LogicalSize(380, 10));
};

// Разворачивание: увеличиваем окно до 480px высоты
async function expandHub() {
  await appWindow.setSize(new LogicalSize(380, 480));
  const hub = document.getElementById('hub-container');
  if (hub) {
    hub.classList.add('expanded');
  }
}

function escapeHtml(text: string): string {
  const map: Record<string, string> = {
    '&': '&amp;',
    '<': '&lt;',
    '>': '&gt;',
    '"': '&quot;',
    "'": '&#039;'
  };
  return text.replace(/[&<>"']/g, m => map[m]);
}

document.addEventListener('DOMContentLoaded', async () => {
  const hub = document.getElementById('hub-container');
  const trigger = document.getElementById('edge-trigger');
  let hoverTimer: number | null = null;
  const HOVER_DELAY = 500; // 0.5 секунды удержания мыши у верха экрана

  // Изначально устанавливаем размер окна 10px
  await appWindow.setSize(new LogicalSize(380, 10));

  if (trigger) {
    trigger.addEventListener('mouseenter', () => {
      if (!hoverTimer) {
        hoverTimer = window.setTimeout(() => {
          expandHub();
        }, HOVER_DELAY);
      }
    });

    trigger.addEventListener('mouseleave', () => {
      if (hoverTimer && (!hub || !hub.classList.contains('expanded'))) {
        clearTimeout(hoverTimer);
        hoverTimer = null;
      }
    });
  }

  if (hub) {
    hub.addEventListener('mouseleave', () => {
      if (hoverTimer) {
        clearTimeout(hoverTimer);
        hoverTimer = null;
      }
      (window as any).collapseHub();
    });
  }

  const clearBtn = document.getElementById('clear-btn');
  if (clearBtn) {
    clearBtn.addEventListener('click', async () => {
      await invoke('clear_history');
      fetchHistory();
    });
  }

  fetchHistory();
  setInterval(fetchHistory, 1500);
});