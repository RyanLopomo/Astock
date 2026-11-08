import { convertedUnits,decimalProductCents,integer,money,parseMoney } from './format';
test('centavos exatos aceitam entrada brasileira e ponto decimal sem multiplicação float',()=>{
 expect(parseMoney('12,50')).toBe(1250);expect(parseMoney('12.50')).toBe(1250);expect(parseMoney('1.234,56')).toBe(123456);expect(parseMoney('1.234')).toBe(123400);expect(()=>parseMoney('1,005')).toThrow();expect(()=>parseMoney('-1')).toThrow();
 expect(money(9007199254740993n)).toBe('R$ 90.071.992.547.409,93');expect(money(-1)).toBe('-R$ 0,01');
});
test('conversões fracionárias e custo de linha são decimais exatos',()=>{
 expect(convertedUnits('0.5',12)).toBe(6);expect(convertedUnits('0.5',1)).toBeNull();expect(convertedUnits('1e3',1)).toBeNull();expect(decimalProductCents('3','0.335')).toBe(101);expect(integer('0')).toBe(0);expect(()=>integer('1.5')).toThrow();expect(()=>integer('0',true)).toThrow();
});
