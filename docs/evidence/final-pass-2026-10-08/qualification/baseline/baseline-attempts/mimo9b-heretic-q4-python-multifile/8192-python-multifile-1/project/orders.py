from pricing import discounted
def total(price, discount_percent, tax_rate):
    subtotal = discounted(price, discount_percent)
    return subtotal * (1 + tax_rate)
