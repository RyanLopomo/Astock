import { act, fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import type { FormEvent } from 'react';
import { beforeEach, vi } from 'vitest';
import { business, type BusinessClient, type InventoryDetail, type ReceiptDetail, type Sku, type Workspace } from './business-api';
import type { PageProps } from './ui';
import { Field, Modal, SkuPicker } from './ui';
import Products, { ProductForm, RegularizeForm, StockForm } from './Products';
import Receipts, { ReceiptView } from './Receipts';
import Inventory from './Inventory';
import Suppliers from './Suppliers';
import WorkspaceApp from './WorkspaceApp';

// Clientes e registros sintéticos restritos a testes. Nenhuma chamada ao SQLite/IPC real.
const sku: Sku = {
  id: 1, productId: 1, name: 'Água UX', description: '', brand: 'TESTE', category: 'Bebidas',
  internalCode: 'AGUA-UX', presentation: '500 ml', purchaseCents: 100, saleCents: 200,
  defaultPack: 1, expiryControl: false, active: true, productActive: true,
  barcodes: ['00123456'], balance: 5, unidentifiedBalance: 5,
};
function workspace(): Workspace {
  return {
    skus: [structuredClone(sku)], suppliers: [], receipts: [], lots: [], inventories: [],
    settings: { shortDays: 7, longDays: 30 }, localDate: '2026-11-08', schemaVersion: 5,
    sqliteVersion: '3.54.1', dataProfile: 'HOMOLOGACAO-UX-TESTE',
  };
}
function receipt(): ReceiptDetail {
  return {
    receipt: {
      id: 1, supplierId: null, supplierName: 'FORNECEDOR SINTÉTICO', status: 'draft', source: 'xml',
      invoiceNumber: '1', invoiceSeries: '1', issuedAt: '2026-11-08T12:00:00-03:00',
      accessKey: '3'.repeat(44), notes: 'Teste UX isolado', revision: 2,
      createdAt: '2026-11-08T12:00:00Z', totalCents: 100,
    },
    items: [{
      id: 1, skuId: 1, supplierCode: '001', description: 'Água do XML de teste', barcode: '00123456',
      taxBarcode: null, quantity: '1', unitsPerPack: 1, units: 1, commercialUnit: 'UN', unitPrice: '1.00',
      totalCents: 100, unitCostCents: 100, roundingDifferenceCents: 0, taxQuantity: '1', taxUnit: 'UN',
      sourceQuantity: '1', sourceUnitPrice: '1.00', sourceTotalCents: 100, sourceAdjustmentCents: 0,
      sourceLots: [], lots: [], candidates: [],
    }],
  };
}
function inventory(): InventoryDetail {
  return {
    inventory: {
      id: 9, name: 'INVENTÁRIO SINTÉTICO UX', status: 'active', scope: 'full', revision: 0,
      createdAt: '2026-11-08T12:00:00Z', closedAt: null, uncountedDecision: null, skuCount: 1,
    },
    lines: [{
      id: 1, skuId: 1, skuName: sku.name, internalCode: sku.internalCode, lotId: null,
      lotLabel: 'Sem lote identificado', expiresOn: null, reference: 5, counted: null, difference: null,
    }], events: [], eventsTotal: 0,
  };
}
function client(data = workspace()): BusinessClient {
  return {
    ...business, available: true, workspace: vi.fn().mockResolvedValue(data),
    receipt: vi.fn().mockResolvedValue(receipt()), saveProduct: vi.fn().mockResolvedValue(sku),
    saveReceiptItem: vi.fn().mockResolvedValue(receipt()), inventory: vi.fn().mockResolvedValue(inventory()),
    inventoryEvents: vi.fn().mockResolvedValue({ items: [], total: 0, page: 0 }),
  };
}
function props(data = workspace()): PageProps {
  return { data, client: client(data), refresh: vi.fn().mockResolvedValue(undefined), navigate: vi.fn() };
}
beforeEach(() => sessionStorage.clear());

test('leitor adiciona códigos com zeros sem submeter cadastro nem duplicar o mesmo código', async () => {
  const p = props(); const user = userEvent.setup(); render(<Products {...p} />);
  await user.click(screen.getByRole('button', { name: 'Novo produto' }));
  const dialog = screen.getByRole('dialog');
  const scanner = within(dialog).getByLabelText('Adicionar código de barras');
  await user.type(scanner, '00000123{Enter}00000456{Enter}00000123{Enter}');
  expect(within(dialog).getByLabelText('Códigos de barras (um por linha)')).toHaveValue('00000123\n00000456');
  expect(scanner).toHaveValue(''); expect(scanner).toHaveFocus();
  expect(p.client.saveProduct).not.toHaveBeenCalled();
});

test('salvar não perde silenciosamente um código digitado que ainda não foi adicionado', async () => {
  const p = props(); const user = userEvent.setup(); render(<Products {...p} />);
  await user.click(screen.getByRole('button', { name: 'Novo produto' }));
  await user.type(screen.getByLabelText('Nome do produto'), 'Produto com código pendente');
  await user.type(screen.getByLabelText('Código interno único'), 'CODIGO-PENDENTE');
  await user.type(screen.getByLabelText('Adicionar código de barras'), '00007890');
  await user.click(screen.getByRole('button', { name: 'Salvar produto' }));
  expect(await screen.findByRole('alert')).toHaveTextContent('Adicione o código digitado');
  expect(screen.getByLabelText('Adicionar código de barras')).toHaveValue('00007890');
  expect(p.client.saveProduct).not.toHaveBeenCalled();
});

test('código compartilhado avisa a associação existente e continua permitido no cadastro', async () => {
  const p = props(); const user = userEvent.setup(); render(<Products {...p} />);
  await user.click(screen.getByRole('button', { name: 'Novo produto' }));
  const dialog = screen.getByRole('dialog');
  await user.type(within(dialog).getByLabelText('Nome do produto'), 'Outra variante');
  await user.type(within(dialog).getByLabelText('Código interno único'), 'OUTRA-UX');
  await user.type(within(dialog).getByLabelText('Adicionar código de barras'), '00123456{Enter}');
  expect(dialog).toHaveTextContent(/Água UX|AGUA-UX/);
  await user.click(within(dialog).getByRole('button', { name: 'Salvar produto' }));
  await waitFor(() => expect(p.client.saveProduct).toHaveBeenCalledWith(expect.objectContaining({ barcodes: ['00123456'] })));
});

test.each(['Escape', 'Cancelar', 'Fechar janela'])('protege o preenchimento ao fechar por %s', async method => {
  const p = props(); const user = userEvent.setup(); render(<Products {...p} />);
  await user.click(screen.getByRole('button', { name: 'Novo produto' }));
  await user.type(screen.getByLabelText('Nome do produto'), 'Preenchimento a preservar');
  if (method === 'Escape') await user.keyboard('{Escape}');
  else await user.click(within(screen.getByRole('dialog')).getByRole('button', { name: method }));
  await user.click(screen.getByRole('button', { name: 'Continuar editando' }));
  expect(screen.getByLabelText('Nome do produto')).toHaveValue('Preenchimento a preservar');
  await user.keyboard('{Escape}');
  await user.click(screen.getByRole('button', { name: 'Descartar alterações' }));
  expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
  expect(p.client.saveProduct).not.toHaveBeenCalled();
});

test('formulário intacto fecha imediatamente e devolve o foco ao botão de origem', async () => {
  const user = userEvent.setup(); render(<Products {...props()} />);
  const trigger = screen.getByRole('button', { name: 'Novo produto' });
  await user.click(trigger); await user.keyboard('{Escape}');
  expect(screen.queryByRole('dialog')).not.toBeInTheDocument(); expect(trigger).toHaveFocus();
});

test('diálogo mantém Tab e Shift+Tab contidos incluindo links acessíveis', async () => {
  const user = userEvent.setup();
  render(<><button>Fora do diálogo</button><Modal title="Conferência de teste" onClose={vi.fn()}><a href="#detalhes">Detalhes</a><button>Última ação</button></Modal></>);
  expect(screen.getByRole('link', { name: 'Detalhes' })).toHaveFocus();
  await user.click(screen.getByRole('button', { name: 'Última ação' }));
  await user.tab(); expect(screen.getByRole('button', { name: 'Fechar janela' })).toHaveFocus();
  await user.tab({ shift: true }); expect(screen.getByRole('button', { name: 'Última ação' })).toHaveFocus();
});

test('rascunho modificado continua protegendo recarregamento mesmo sob cadastro rápido', () => {
  render(<Modal title="Editor suspenso de teste" onClose={vi.fn()} dirty suspended><input aria-label="Rascunho sintético" /></Modal>);
  const event = new Event('beforeunload', { cancelable: true });
  window.dispatchEvent(event);
  expect(event.defaultPrevented).toBe(true);
});

test('não duplica cadastro em voo, bloqueia fechamento e preserva dados quando salvar falha', async () => {
  const p = props(); const onSaved = vi.fn(); const onClose = vi.fn();
  let rejectSave!: (error: unknown) => void;
  p.client.saveProduct = vi.fn().mockImplementation(() => new Promise<Sku>((_resolve, reject) => { rejectSave = reject; }));
  const user = userEvent.setup(); render(<ProductForm client={p.client} onSaved={onSaved} onClose={onClose} />);
  await user.type(screen.getByLabelText('Nome do produto'), 'Cadastro recuperável');
  await user.type(screen.getByLabelText('Código interno único'), 'RECUPERAR');
  const form = screen.getByRole('button', { name: 'Salvar produto' }).closest('form')!;
  fireEvent.submit(form); fireEvent.submit(form);
  expect(p.client.saveProduct).toHaveBeenCalledTimes(1);
  expect(screen.getByRole('button', { name: 'Salvando…' })).toBeDisabled();
  await user.keyboard('{Escape}'); expect(onClose).not.toHaveBeenCalled();
  await act(async () => rejectSave({ message: 'Banco ocupado. Tente novamente.' }));
  expect(await screen.findByRole('alert')).toHaveTextContent('Banco ocupado');
  expect(screen.getByLabelText('Nome do produto')).toHaveValue('Cadastro recuperável');
  expect(onSaved).not.toHaveBeenCalled();
});

test('falha ao atualizar catálogo depois de gravar não reenvia o cadastro confirmado', async () => {
  const p = props(); const onSaved = vi.fn().mockRejectedValueOnce(new Error('Atualização indisponível.')).mockResolvedValue(undefined);
  const user = userEvent.setup(); render(<ProductForm client={p.client} onSaved={onSaved} onClose={vi.fn()} />);
  await user.type(screen.getByLabelText('Nome do produto'), 'Produto confirmado');
  await user.type(screen.getByLabelText('Código interno único'), 'CONFIRMADO');
  await user.click(screen.getByRole('button', { name: 'Salvar produto' }));
  await user.click(await screen.findByRole('button', { name: 'Atualizar catálogo' }));
  await waitFor(() => expect(onSaved).toHaveBeenCalledTimes(2));
  expect(p.client.saveProduct).toHaveBeenCalledTimes(1);
});

test('ajuda do campo fica associada à descrição acessível do controle', () => {
  render(<Field label="Código" hint="Zeros iniciais são preservados."><input /></Field>);
  expect(screen.getByLabelText('Código')).toHaveAccessibleDescription('Zeros iniciais são preservados.');
});

test('Enter no seletor encontra código exato sem submeter o formulário pai', async () => {
  const onChange = vi.fn(); const onSubmit = vi.fn((event: FormEvent) => event.preventDefault());
  const user = userEvent.setup();
  render(<form onSubmit={onSubmit}><SkuPicker skus={[sku]} value={null} onChange={onChange} /><button type="submit">Salvar teste</button></form>);
  await user.type(screen.getByLabelText('Buscar produto / sku'), '00123456{Enter}');
  expect(onChange).toHaveBeenCalledWith(1); expect(onSubmit).not.toHaveBeenCalled();
});

test('Enter em código ambíguo não escolhe variante nem submete automaticamente', async () => {
  const onChange = vi.fn(); const onSubmit = vi.fn((event: FormEvent) => event.preventDefault());
  const other = { ...sku, id: 2, internalCode: 'OUTRA-UX', presentation: 'Outra variante' };
  const user = userEvent.setup();
  render(<form onSubmit={onSubmit}><SkuPicker skus={[sku, other]} value={null} onChange={onChange} /><button type="submit">Salvar teste</button></form>);
  await user.type(screen.getByLabelText('Buscar produto / sku'), '00123456{Enter}');
  expect(onChange).toHaveBeenCalledWith(null); expect(onChange).not.toHaveBeenCalledWith(expect.any(Number));
  expect(onSubmit).not.toHaveBeenCalled();
  expect(screen.getByRole('option', { name: /OUTRA-UX/ })).toBeVisible();
});

test('código desconhecido limpa a seleção antiga para não receber outro SKU por engano', async () => {
  const onChange = vi.fn(); const user = userEvent.setup();
  render(<SkuPicker skus={[sku]} value={1} onChange={onChange} />);
  await user.type(screen.getByLabelText('Buscar produto / sku'), 'CODIGO-DESCONHECIDO{Enter}');
  expect(onChange).toHaveBeenCalledWith(null); expect(onChange).not.toHaveBeenCalledWith(expect.any(Number));
  expect(screen.getByRole('status')).toHaveTextContent('Nenhum código exato encontrado');
});

test('consulta de produtos mantém busca, categoria e inativos ao sair e voltar', async () => {
  const data = workspace(); const api = client(data); const user = userEvent.setup();
  render(<WorkspaceApp client={api} />); await screen.findByRole('heading', { name: 'Visão geral' });
  await user.click(screen.getByRole('button', { name: /02\s*Produtos/ }));
  await user.type(screen.getByLabelText('Buscar produtos'), 'Água');
  await user.selectOptions(screen.getByLabelText('Categoria'), 'Bebidas');
  await user.click(screen.getByLabelText('Mostrar inativos'));
  await user.click(screen.getByRole('button', { name: /03\s*Recebimentos/ }));
  await user.click(screen.getByRole('button', { name: /02\s*Produtos/ }));
  expect(screen.getByLabelText('Buscar produtos')).toHaveValue('Água');
  expect(screen.getByLabelText('Categoria')).toHaveValue('Bebidas');
  expect(screen.getByLabelText('Mostrar inativos')).toBeChecked();
});

test('busca sem resultado oferece limpar filtros sem orientar criar o primeiro produto', async () => {
  const user = userEvent.setup(); render(<Products {...props()} />);
  await user.type(screen.getByLabelText('Buscar produtos'), 'produto inexistente');
  expect(screen.queryByText(/Cadastre o primeiro produto/)).not.toBeInTheDocument();
  await user.click(screen.getByRole('button', { name: 'Limpar filtros' }));
  expect(screen.getByLabelText('Buscar produtos')).toHaveValue('');
  expect(screen.getByText('Água UX')).toBeVisible();
});

test('consulta continua funcionando quando armazenamento de sessão está indisponível', async () => {
  vi.spyOn(Storage.prototype, 'getItem').mockImplementation(() => { throw new Error('Storage bloqueado'); });
  vi.spyOn(Storage.prototype, 'setItem').mockImplementation(() => { throw new Error('Storage bloqueado'); });
  const user = userEvent.setup(); render(<Products {...props()} />);
  await user.type(screen.getByLabelText('Buscar produtos'), 'Água');
  expect(screen.getByText('Água UX')).toBeVisible();
});

test('inventário retomado recebe foco no leitor logo após carregar a sessão', async () => {
  const p = props(); p.data.inventories = [inventory().inventory]; render(<Inventory {...p} />);
  const field = await screen.findByLabelText('Código de barras ou SKU');
  await waitFor(() => expect(field).toHaveFocus());
});

async function editReceiptDraft(user: ReturnType<typeof userEvent.setup>) {
  await user.click(screen.getByRole('button', { name: 'Associar / conferir' }));
  const dialog = screen.getByRole('dialog');
  for (const [label, value] of [
    ['Quantidade comercial (decimal)', '3'], ['Unidades de estoque por embalagem', '12'],
    ['Valor unitário comercial (R$, até 10 casas)', '12.00'], ['Custo total recebido (R$)', '36,00'],
  ]) {
    const field = within(dialog).getByLabelText(label); await user.clear(field); await user.type(field, value);
  }
  await user.click(within(dialog).getByRole('button', { name: 'Adicionar lote' }));
  await user.type(within(dialog).getByLabelText('Identificação do lote'), 'LOTE-UX');
  return dialog;
}
function expectReceiptDraft(dialog: HTMLElement) {
  expect(within(dialog).getByLabelText('Quantidade comercial (decimal)')).toHaveValue('3');
  expect(within(dialog).getByLabelText('Unidades de estoque por embalagem')).toHaveValue(12);
  expect(within(dialog).getByLabelText('Valor unitário comercial (R$, até 10 casas)')).toHaveValue('12.00');
  expect(within(dialog).getByLabelText('Custo total recebido (R$)')).toHaveValue('36,00');
  expect(within(dialog).getByLabelText('Identificação do lote')).toHaveValue('LOTE-UX');
  expect(within(dialog).getByLabelText('Quantidade no lote')).toHaveValue(36);
}

test('cancelar cadastro rápido preserva quantidade, conversão, preço, total e lote do recebimento', async () => {
  const p = props(); const user = userEvent.setup(); render(<ReceiptView {...p} id={1} initial={receipt()} />);
  const editor = await editReceiptDraft(user);
  await user.click(within(editor).getByRole('button', { name: 'Cadastrar produto novo' }));
  const quick = screen.getByRole('dialog');
  expect(within(quick).getByRole('heading', { name: 'Cadastrar produto do recebimento' })).toBeVisible();
  await user.click(within(quick).getByRole('button', { name: 'Cancelar' }));
  expectReceiptDraft(screen.getByRole('dialog'));
  expect(p.client.saveReceiptItem).not.toHaveBeenCalled();
});

test('salvar cadastro rápido associa o novo SKU e conserva o rascunho antes da conferência explícita', async () => {
  const p = props(); const nextSku = { ...sku, id: 2, productId: 2, internalCode: 'NOVO-UX', defaultPack: 4 };
  p.client.saveProduct = vi.fn().mockResolvedValue(nextSku);
  p.refresh = vi.fn().mockImplementation(async () => { if (!p.data.skus.some(row => row.id === 2)) p.data.skus.push(nextSku); });
  const user = userEvent.setup(); render(<ReceiptView {...p} id={1} initial={receipt()} />);
  const editor = await editReceiptDraft(user);
  await user.click(within(editor).getByRole('button', { name: 'Cadastrar produto novo' }));
  await user.type(screen.getByLabelText('Código interno único'), 'NOVO-UX');
  await user.click(screen.getByRole('button', { name: 'Salvar produto' }));
  await screen.findByRole('heading', { name: 'Associar e conferir item' });
  const resumed = screen.getByRole('dialog'); expectReceiptDraft(resumed);
  expect(within(resumed).getByLabelText('Produto / SKU')).toHaveValue('2');
  expect(p.client.saveReceiptItem).not.toHaveBeenCalled();
  await user.click(within(resumed).getByRole('button', { name: 'Salvar conferência do item' }));
  await waitFor(() => expect(p.client.saveReceiptItem).toHaveBeenCalledWith(expect.objectContaining({
    receiptId: 1, expectedRevision: 2, skuId: 2, quantity: '3', unitsPerPack: 12,
    unitPrice: '12.00', totalCents: 3600, lots: [expect.objectContaining({ label: 'LOTE-UX', quantity: 36 })],
  })));
});

test('item confirmado com falha na atualização repete somente a consulta sem regravar a conferência', async () => {
  const p = props(); const saved = receipt(); saved.receipt.revision = 3;
  p.client.saveReceiptItem = vi.fn().mockResolvedValue(saved);
  p.refresh = vi.fn().mockRejectedValueOnce(new Error('Atualização local indisponível.')).mockResolvedValue(undefined);
  const user = userEvent.setup(); render(<ReceiptView {...p} id={1} initial={receipt()} />);
  const editor = await editReceiptDraft(user);
  await user.click(within(editor).getByRole('button', { name: 'Salvar conferência do item' }));
  await within(editor).findByRole('button', { name: 'Atualizar conferência' });
  expect(within(editor).getByRole('alert')).toHaveTextContent('Atualização local indisponível');
  expect(within(editor).getByLabelText('Quantidade comercial (decimal)')).toBeDisabled();
  expect(p.client.saveReceiptItem).toHaveBeenCalledTimes(1);
  expect(p.client.saveReceiptItem).toHaveBeenCalledWith(expect.objectContaining({ expectedRevision: 2, quantity: '3', unitsPerPack: 12 }));
  await user.click(within(editor).getByRole('button', { name: 'Atualizar conferência' }));
  await waitFor(() => expect(screen.queryByRole('dialog')).not.toBeInTheDocument());
  expect(p.client.saveReceiptItem).toHaveBeenCalledTimes(1); expect(p.refresh).toHaveBeenCalledTimes(2);
});

test('recebimento criado com falha na atualização não cria outro rascunho na nova tentativa', async () => {
  const p = props(); p.client.createReceipt = vi.fn().mockResolvedValue(receipt());
  p.refresh = vi.fn().mockRejectedValueOnce(new Error('Lista de recebimentos indisponível.')).mockResolvedValue(undefined);
  const user = userEvent.setup(); render(<Receipts {...p} />);
  await user.click(screen.getByRole('button', { name: 'Novo recebimento' }));
  const dialog = screen.getByRole('dialog');
  await user.type(within(dialog).getByLabelText('Observação'), 'Rascunho UX confirmado');
  await user.click(within(dialog).getByRole('button', { name: 'Criar rascunho' }));
  await within(dialog).findByRole('button', { name: 'Atualizar recebimento' });
  expect(within(dialog).getByLabelText('Observação')).toBeDisabled();
  expect(p.client.createReceipt).toHaveBeenCalledTimes(1);
  expect(p.client.createReceipt).toHaveBeenCalledWith({ supplierId: null, notes: 'Rascunho UX confirmado' });
  await user.click(within(dialog).getByRole('button', { name: 'Atualizar recebimento' }));
  await waitFor(() => expect(screen.queryByRole('dialog')).not.toBeInTheDocument());
  expect(p.client.createReceipt).toHaveBeenCalledTimes(1); expect(p.refresh).toHaveBeenCalledTimes(2);
  expect(screen.getByRole('heading', { name: /Recebimento #1/ })).toBeVisible();
});

test('inventário criado com falha na atualização retoma a sessão confirmada sem criar outra', async () => {
  const p = props(); p.client.createInventory = vi.fn().mockResolvedValue(inventory());
  p.refresh = vi.fn().mockRejectedValueOnce(new Error('Lista de inventários indisponível.')).mockResolvedValue(undefined);
  const user = userEvent.setup(); render(<Inventory {...p} />);
  await user.click(screen.getByRole('button', { name: 'Novo inventário' }));
  const dialog = screen.getByRole('dialog');
  await user.type(within(dialog).getByLabelText('Identificação da contagem'), 'CONTAGEM SINTÉTICA UX');
  await user.click(within(dialog).getByRole('button', { name: 'Iniciar contagem' }));
  await within(dialog).findByRole('button', { name: 'Atualizar inventário' });
  expect(within(dialog).getByLabelText('Identificação da contagem')).toBeDisabled();
  expect(p.client.createInventory).toHaveBeenCalledTimes(1);
  expect(p.client.createInventory).toHaveBeenCalledWith({ name: 'CONTAGEM SINTÉTICA UX', skuIds: null });
  await user.click(within(dialog).getByRole('button', { name: 'Atualizar inventário' }));
  await waitFor(() => expect(screen.queryByRole('dialog')).not.toBeInTheDocument());
  expect(p.client.createInventory).toHaveBeenCalledTimes(1); expect(p.refresh).toHaveBeenCalledTimes(2);
  expect(await screen.findByLabelText('Código de barras ou SKU')).toHaveFocus();
});

test('fornecedor confirmado com falha na atualização não repete a gravação do cadastro novo', async () => {
  const p = props(); p.client.saveSupplier = vi.fn().mockResolvedValue({ id: 7, name: 'FORNECEDOR SINTÉTICO UX', taxId: '', contact: '', active: true });
  p.refresh = vi.fn().mockRejectedValueOnce(new Error('Lista de fornecedores indisponível.')).mockResolvedValue(undefined);
  const user = userEvent.setup(); render(<Suppliers {...p} />);
  await user.click(screen.getByRole('button', { name: 'Novo fornecedor' }));
  const dialog = screen.getByRole('dialog');
  await user.type(within(dialog).getByLabelText('Nome do fornecedor'), 'FORNECEDOR SINTÉTICO UX');
  await user.click(within(dialog).getByRole('button', { name: 'Salvar fornecedor' }));
  await within(dialog).findByRole('button', { name: 'Atualizar fornecedores' });
  expect(within(dialog).getByLabelText('Nome do fornecedor')).toBeDisabled();
  expect(p.client.saveSupplier).toHaveBeenCalledTimes(1);
  await user.click(within(dialog).getByRole('button', { name: 'Atualizar fornecedores' }));
  await waitFor(() => expect(screen.queryByRole('dialog')).not.toBeInTheDocument());
  expect(p.client.saveSupplier).toHaveBeenCalledTimes(1); expect(p.refresh).toHaveBeenCalledTimes(2);
});

test.each(['issue', 'adjust'] as const)('movimento %s confirmado com falha na atualização não é reenviado nem tratado como não salvo', async mode => {
  const p = props(); const onClose = vi.fn();
  p.client.issue = vi.fn().mockResolvedValue(undefined); p.client.adjust = vi.fn().mockResolvedValue(undefined);
  p.client.planIssue = vi.fn().mockResolvedValue({ skuId: 1, quantity: 1, allocations: [{ lotId: null, quantity: 1 }], warnings: [] });
  p.refresh = vi.fn().mockRejectedValueOnce(new Error('Saldo atualizado indisponível.')).mockResolvedValue(undefined);
  const user = userEvent.setup(); render(<StockForm {...p} sku={sku} mode={mode} onClose={onClose} />);
  if (mode === 'issue') await user.click(screen.getByLabelText('Incluir saldos sem validade após os lotes datados'));
  await user.type(screen.getByLabelText('Motivo obrigatório'), 'Movimento sintético UX');
  await user.click(screen.getByRole('button', { name: mode === 'issue' ? 'Conferir retirada' : 'Conferir ajuste' }));
  await user.click(screen.getByRole('button', { name: mode === 'issue' ? 'Confirmar saída' : 'Confirmar ajuste' }));
  const retry = await screen.findByRole('button', { name: 'Atualizar estoque' });
  expect(screen.getByRole('alert')).toHaveTextContent('Saldo atualizado indisponível');
  const back = screen.queryByRole('button', { name: 'Voltar' }); if (back) expect(back).toBeDisabled();
  const unload = new Event('beforeunload', { cancelable: true }); window.dispatchEvent(unload);
  expect(unload.defaultPrevented).toBe(false);
  const write = mode === 'issue' ? p.client.issue : p.client.adjust;
  expect(write).toHaveBeenCalledTimes(1); expect(p.refresh).toHaveBeenCalledTimes(1);
  await user.click(retry); await waitFor(() => expect(onClose).toHaveBeenCalledTimes(1));
  expect(write).toHaveBeenCalledTimes(1); expect(p.refresh).toHaveBeenCalledTimes(2);
});

test('regularização confirmada com falha na atualização repete só a consulta e não os movimentos compensatórios', async () => {
  const p = props(); const onClose = vi.fn(); p.client.regularize = vi.fn().mockResolvedValue(undefined);
  p.refresh = vi.fn().mockRejectedValueOnce(new Error('Lotes atualizados indisponíveis.')).mockResolvedValue(undefined);
  const user = userEvent.setup(); render(<RegularizeForm {...p} onClose={onClose} />);
  await user.selectOptions(screen.getByLabelText('Produto / SKU'), '1');
  await user.type(screen.getByLabelText('Identificação do lote'), 'LOTE-REG-UX');
  await user.type(screen.getByLabelText('Motivo da regularização'), 'Identificação sintética UX');
  await user.click(screen.getByRole('button', { name: 'Conferir regularização' }));
  await user.click(screen.getByRole('button', { name: 'Confirmar regularização' }));
  const retry = await screen.findByRole('button', { name: 'Atualizar estoque' });
  expect(screen.getByRole('alert')).toHaveTextContent('Lotes atualizados indisponíveis');
  const back = screen.queryByRole('button', { name: 'Voltar' }); if (back) expect(back).toBeDisabled();
  const unload = new Event('beforeunload', { cancelable: true }); window.dispatchEvent(unload);
  expect(unload.defaultPrevented).toBe(false);
  expect(p.client.regularize).toHaveBeenCalledTimes(1);
  await user.click(retry); await waitFor(() => expect(onClose).toHaveBeenCalledTimes(1));
  expect(p.client.regularize).toHaveBeenCalledTimes(1); expect(p.refresh).toHaveBeenCalledTimes(2);
});

test('Escape após ajuste confirmado fecha sem oferecer descartar um movimento já aplicado', async () => {
  const p = props(); p.client.adjust = vi.fn().mockResolvedValue(undefined);
  p.refresh = vi.fn().mockRejectedValue(new Error('Consulta de saldos indisponível.'));
  const user = userEvent.setup(); render(<Products {...p} />);
  await user.click(screen.getByRole('button', { name: 'Ajuste' }));
  await user.type(screen.getByLabelText('Motivo obrigatório'), 'Ajuste sintético confirmado');
  await user.click(screen.getByRole('button', { name: 'Conferir ajuste' }));
  await user.click(screen.getByRole('button', { name: 'Confirmar ajuste' }));
  await screen.findByRole('button', { name: 'Atualizar estoque' });
  await user.keyboard('{Escape}');
  expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
  expect(screen.queryByText('Descartar alterações?')).not.toBeInTheDocument();
  expect(p.client.adjust).toHaveBeenCalledTimes(1); expect(p.refresh).toHaveBeenCalledTimes(1);
});
