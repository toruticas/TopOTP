import { invoke } from '@tauri-apps/api/core';
import type {
  AccountInput,
  AccountSummary,
  Code,
  ImportPreview,
  VaultStatus,
} from './generated';
export type * from './generated';
export const api = {
  status: () => invoke<VaultStatus>('vault_status'),
  unlock: () => invoke<VaultStatus>('unlock_vault'),
  lock: () => invoke<void>('lock_vault'),
  hide: () => invoke<void>('hide_panel'),
  interact: (epoch: string) => invoke<void>('user_interaction', { epoch }),
  list: (epoch: string) => invoke<AccountSummary[]>('list_accounts', { epoch }),
  codes: (epoch: string) => invoke<Code[]>('get_totp_codes', { epoch }),
  save: (epoch: string, accountId: string | null, input: AccountInput) =>
    invoke<void>('save_account', { epoch, accountId, input }),
  uri: (epoch: string, uri: string) =>
    invoke<ImportPreview>('preview_uri_account', { epoch, uri }),
  delete: (epoch: string, accountId: string) =>
    invoke<void>('delete_account', { epoch, accountId }),
  next: (epoch: string, accountId: string) =>
    invoke<Code>('generate_hotp', { epoch, accountId }),
  copy: (epoch: string, accountId: string) =>
    invoke<void>('copy_code', { epoch, accountId }),
  order: (epoch: string, ids: string[], pinned: string[]) =>
    invoke<void>('set_account_order', { epoch, ids, pinned }),
  import: (epoch: string) => invoke<ImportPreview>('begin_import', { epoch }),
  passwords: (
    epoch: string,
    previewId: string,
    passwords: Record<string, string>,
  ) =>
    invoke<ImportPreview>('provide_import_passwords', {
      epoch,
      previewId,
      passwords,
    }),
  commit: (
    epoch: string,
    previewId: string,
    selected: string[],
    acknowledgeSkipped: boolean,
  ) =>
    invoke<void>('commit_import', {
      epoch,
      previewId,
      selected,
      acknowledgeSkipped,
    }),
  cancel: (epoch: string) => invoke<void>('cancel_import', { epoch }),
  export: (
    epoch: string,
    ids: string[],
    encrypted: boolean,
    password: string,
    confirmation: string,
    acknowledgePlaintext: boolean,
  ) =>
    invoke<void>('export_accounts', {
      epoch,
      request: { ids, encrypted, password, confirmation, acknowledgePlaintext },
    }),
};
export function errorMessage(error: unknown): string {
  const code =
    typeof error === 'object' && error !== null && 'code' in error
      ? String(error.code)
      : '';
  const messages: Record<string, string> = {
    Locked: 'O cofre foi bloqueado. Desbloqueie para continuar.',
    Cancelled: 'Operação cancelada.',
    InvalidInput: 'Confira os campos e confirmações.',
    UnsupportedFormat:
      'Formato não suportado. Reexporte o backup pela extensão atual.',
    StorageUnavailable:
      'Não foi possível acessar o Keychain ou gravar o arquivo.',
    DeletionPending:
      'A conta saiu da lista, mas a exclusão no Keychain ainda não foi confirmada. Bloqueie e desbloqueie o cofre para tentar novamente.',
    NotFound: 'Conta ou prévia não encontrada.',
    CounterExhausted: 'O contador atingiu o limite suportado.',
    InvalidPassword: 'Senha do backup incorreta.',
    LimitExceeded:
      'O arquivo ou processamento excede os limites de segurança. Na exportação, selecione menos contas ou reduza nomes muito longos.',
    Conflict: 'Os dados mudaram. Revise a operação.',
  };
  return messages[code] ?? 'Não foi possível concluir a operação.';
}
