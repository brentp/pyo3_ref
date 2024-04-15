Genotype = {}
Genotype.__index = Genotype
Allele = {}
Allele.__index = Allele

function Allele.new(value)
    local o = {}
    setmetatable(o, Allele)
    o.value = value
    return o
end

function Allele:val()
    return bit32.rshift(self.value, 1) - 1
end

function Allele:phased()
    return bit32.band(rawget(self, "value"), 1)
end

function Allele:set_phase(phase)
    local newv = bit32.bor(self.value, tonumber(phase))
    rawset(self, "value", newv)
end

function Allele:set_allele(val)
    local newv = bit32.bor(bit32.lshift(val + 1, 1), self:phased())
    rawset(self, "value", newv)
end

function Allele:__tostring()
    return self:val()
end

function Genotype.new(raw_alleles)
    local self = setmetatable({ alleles = raw_alleles }, Genotype)
    return self
end

function Genotype:__index(i)
    print("get index:", i, "type:", type(i))
    return Allele.new(self.alleles[i])
end

-- return error on newindex
function Genotype:__newindex(i, v)
    if i > #self.alleles then
        error("Genotype: cant currently append")
    end
    self.alleles[i] = v.value
end

function Genotype:__tostring()
    local str = { self[1]:val() }
    local porder = { '/', '|' }
    for i = 2, #self.alleles do
        str[#str + 1] = porder[self[i]:phased() + 1]
        str[#str + 1] = self[i]:val()
    end

    return table.concat(str, "")
end

function Genotype:__len()
    return #self.alleles
end

local g1 = Genotype.new({ 2, 5 })
local g2 = Genotype.new({ 4, 4, 4 })
print(#g1)

print(g1)
print(g2)
a = g2[2]
print("a:", a)
a:set_phase(1)
a:set_allele(2)
print("set it")
g2[2] = a
print("after phasing")
print(g2)

print(table.concat(g2.alleles, ","))
print(table.concat(g1.alleles, ","))