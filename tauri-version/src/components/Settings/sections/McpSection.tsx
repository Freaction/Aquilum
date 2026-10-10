import { useCallback, useEffect, useRef, useState } from 'react';
import {
  applyMcpSettings,
  createMcpToken,
  getMcpStatus,
  type McpStatus,
} from '../../../modules/mcp';
import type { AppConfig, McpSettings } from '../../../modules/settings';
import { Input } from '../../Common/Input';
import { t } from '../../../i18n';
import { Switch } from '../../Common/Switch';
import { TextButton } from '../../Common/TextButton';
import { NumberControl } from '../controls/NumberControl';
import { Row } from '../Row';
import { Section } from '../Section';
import { useSettingsPersist } from '../useSettingsPersist';
import './McpSection.css';

interface Snippet {
  title: string;
  hint: string;
  code: string;
}

function snippets(settings: McpSettings, executable: string): Snippet[] {
  const url = `http://127.0.0.1:${settings.port}/mcp`;
  const bearer = `Bearer ${settings.token}`;
  return [
    {
      title: 'Claude Code',
      hint: t('settings.mcp.claudeHint'),
      code: `claude mcp add --transport http aquilum ${url} --header "Authorization: ${bearer}"`,
    },
    {
      title: 'Codex CLI',
      hint: t('settings.mcp.codexHint'),
      code: [
        '[mcp_servers.aquilum]',
        `command = "${executable.replace(/\\/g, '\\\\')}"`,
        'args = ["--mcp-stdio"]',
      ].join('\n'),
    },
    {
      title: 'Gemini CLI',
      hint: t('settings.mcp.geminiHint'),
      code: JSON.stringify(
        {
          mcpServers: {
            aquilum: { httpUrl: url, headers: { Authorization: bearer } },
          },
        },
        null,
        2,
      ),
    },
  ];
}

function StatusLabel({ status, enabled }: { status: McpStatus | null; enabled: boolean }) {
  if (status?.error) {
    return <span className="q-mcp-status q-mcp-status--error">{status.error}</span>;
  }
  if (status?.running) {
    return (
      <span className="q-mcp-status q-mcp-status--running">
        {t('settings.mcp.running', { port: status.port })}
      </span>
    );
  }
  return (
    <span className="q-mcp-status">{enabled ? t('settings.mcp.starting') : t('settings.mcp.off')}</span>
  );
}

function useMcpRestart(onStatus: (status: McpStatus) => void) {
  const runningRef = useRef(false);
  const queuedRef = useRef(false);

  return useCallback(async () => {
    if (runningRef.current) {
      queuedRef.current = true;
      return;
    }
    runningRef.current = true;
    try {
      do {
        queuedRef.current = false;
        try {
          onStatus(await applyMcpSettings());
        } catch (error) {
          console.error('Failed to apply MCP settings', error);
        }
      } while (queuedRef.current);
    } finally {
      runningRef.current = false;
    }
  }, [onStatus]);
}

export function McpSection({ config }: { config: AppConfig }) {
  const { persist } = useSettingsPersist();
  const [status, setStatus] = useState<McpStatus | null>(null);
  const [copied, setCopied] = useState<string | null>(null);
  const [portDraft, setPortDraft] = useState<number | null>(null);
  const [tokenDraft, setTokenDraft] = useState<string | null>(null);
  const restart = useMcpRestart(setStatus);
  const mcp = config.mcp;

  useEffect(() => {
    void getMcpStatus().then(setStatus).catch((error) => {
      console.error('Failed to read MCP status', error);
    });
  }, []);

  const patch = async (partial: Partial<McpSettings>) => {
    if (await persist({ ...config, mcp: { ...mcp, ...partial } })) await restart();
  };

  const commitPort = () => {
    if (portDraft === null) return;
    setPortDraft(null);
    if (portDraft !== mcp.port) void patch({ port: portDraft });
  };

  const commitToken = () => {
    if (tokenDraft === null) return;
    setTokenDraft(null);
    if (tokenDraft !== mcp.token) void patch({ token: tokenDraft });
  };

  const regenerateToken = () => {
    setTokenDraft(null);
    void patch({ token: createMcpToken() });
  };

  const enable = (enabled: boolean) => {
    void patch({ enabled, token: mcp.token || createMcpToken() });
  };

  const copy = (snippet: Snippet) => {
    void navigator.clipboard.writeText(snippet.code)
      .then(() => setCopied(snippet.title))
      .catch((error) => console.error('Failed to copy snippet', error));
  };

  return (
    <>
      <Section title={t('settings.mcp.section')}>
        <Row
          label={t('settings.mcp.access')}
          description={t('settings.mcp.accessHint')}
        >
          <Switch label={t('settings.mcp.switchLabel')} checked={mcp.enabled} onChange={enable} />
        </Row>
        <Row label={t('settings.mcp.status')}>
          <StatusLabel status={status} enabled={mcp.enabled} />
        </Row>
        <Row label={t('settings.mcp.port')}>
          <NumberControl
            ariaLabel={t('settings.mcp.port')}
            value={portDraft ?? mcp.port}
            min={1024}
            max={65535}
            step={1}
            onChange={setPortDraft}
            onCommit={commitPort}
          />
        </Row>
        <Row label={t('settings.mcp.token')} description={t('settings.mcp.tokenHint')}>
          <Input
            value={tokenDraft ?? mcp.token}
            ariaLabel={t('settings.mcp.token')}
            placeholder={t('settings.mcp.tokenPlaceholder')}
            onChange={setTokenDraft}
            onBlur={commitToken}
            onKeyDown={(event) => {
              if (event.key !== 'Enter') return false;
              event.preventDefault();
              commitToken();
              return true;
            }}
          />
        </Row>
        <Row label={t('settings.mcp.newToken')} description={t('settings.mcp.newTokenHint')}>
          <TextButton onClick={regenerateToken}>
            {t('settings.mcp.generate')}
          </TextButton>
        </Row>
        <Row
          label={t('settings.mcp.allowWrite')}
          description={t('settings.mcp.allowWriteHint')}
        >
          <Switch
            label={t('settings.mcp.allowWriteLabel')}
            checked={mcp.allowWrite}
            onChange={(allowWrite) => void patch({ allowWrite })}
          />
        </Row>
      </Section>

      {mcp.enabled && mcp.token ? (
        <Section title={t('settings.mcp.connection')}>
          {snippets(mcp, status?.executable ?? '').map((snippet) => (
            <div className="q-mcp-snippet" key={snippet.title}>
              <Row label={snippet.title} description={snippet.hint}>
                <TextButton onClick={() => copy(snippet)}>
                  {copied === snippet.title ? t('settings.mcp.copied') : t('settings.mcp.copy')}
                </TextButton>
              </Row>
              <pre className="q-mcp-snippet__code">{snippet.code}</pre>
            </div>
          ))}
        </Section>
      ) : null}
    </>
  );
}
