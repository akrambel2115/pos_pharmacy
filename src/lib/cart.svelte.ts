import { calculateRemainingTreatmentDays } from "./utils";

export interface CartItem {
  drug: any;
  quantity: number;
}

class CartStore {
  cart = $state<CartItem[]>([]);
  customerInput = $state("");
  selectedCustomer = $state<any | null>(null);
  lastInvoice = $state<any | null>(null);
  patientName = $state("");
  doctorName = $state("");
  drugSearchQuery = $state("");
  treatmentDaysInput = $state<number | string>(30);

  get cartTotal(): number {
    return this.cart.reduce((sum, item) => sum + (item.drug.price_per_item_da || 0) * item.quantity, 0);
  }

  get requiresPrescription(): boolean {
    return this.cart.some(item => item.drug?.requires_prescription);
  }

  get lastInvoiceTreatmentStatus() {
    if (!this.lastInvoice || !this.lastInvoice.treatment_period_days || !this.lastInvoice.created_at) {
      return null;
    }
    return calculateRemainingTreatmentDays(this.lastInvoice.created_at, this.lastInvoice.treatment_period_days);
  }

  addToCart(drug: any) {
    const existing = this.cart.find(item => item.drug.id === drug.id);
    if (existing) {
      existing.quantity += 1;
    } else {
      this.cart = [...this.cart, { drug, quantity: 1 }];
    }
  }

  removeFromCart(drugId: number) {
    this.cart = this.cart.filter(item => item.drug.id !== drugId);
  }

  updateQuantity(drugId: number, delta: number) {
    const item = this.cart.find(item => item.drug.id === drugId);
    if (item) {
      item.quantity += delta;
      if (item.quantity <= 0) {
        this.removeFromCart(drugId);
      }
    }
  }

  selectCustomer(customer: any, lastInvoice: any = null) {
    this.selectedCustomer = customer;
    this.customerInput = customer.name;
    this.lastInvoice = lastInvoice;
  }

  clearCustomer() {
    this.selectedCustomer = null;
    this.customerInput = "";
    this.lastInvoice = null;
  }

  clearCart() {
    this.cart = [];
    this.patientName = "";
    this.doctorName = "";
    this.drugSearchQuery = "";
    this.clearCustomer();
  }
}

export const cartStore = new CartStore();
