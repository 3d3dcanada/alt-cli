from pricing import discounted
def total(price, discount_percent, tax_rate):
    subtotal = discounted(price, discount_percent)
    return subtotal + price * tax_rate
