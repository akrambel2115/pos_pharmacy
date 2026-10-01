import { invoke } from "@tauri-apps/api/core";

export interface CashierPermissions {
  // Page / Tab Access
  access_stock: boolean;
  access_invoices: boolean;
  access_loans: boolean;
  access_history: boolean;
  access_patients: boolean;

  // Specific Actions
  can_give_loans: boolean;
  can_settle_loans: boolean;
  can_add_drug: boolean;
  can_edit_drug: boolean;
  can_delete_drug: boolean;
  can_delete_patient: boolean;
}

export const defaultCashierPermissions: CashierPermissions = {
  access_stock: false,
  access_invoices: false,
  access_loans: false,
  access_history: false,
  access_patients: false,
  can_give_loans: false,
  can_settle_loans: false,
  can_add_drug: false,
  can_edit_drug: false,
  can_delete_drug: false,
  can_delete_patient: false,
};

class PermissionsStore {
  permissions = $state<CashierPermissions>({ ...defaultCashierPermissions });
  loaded = $state(false);

  async load() {
    try {
      const settings = await invoke<any>("get_settings");
      if (settings && settings.cashier_permissions) {
        try {
          const parsed = JSON.parse(settings.cashier_permissions);
          this.permissions = { ...defaultCashierPermissions, ...parsed };
        } catch {
          this.permissions = { ...defaultCashierPermissions };
        }
      }
      this.loaded = true;
    } catch (e) {
      console.error("Failed to load cashier permissions:", e);
    }
  }

  isAllowed(userRole: "cashier" | "admin" | null | string, action: keyof CashierPermissions): boolean {
    if (userRole === "admin") return true;
    return !!this.permissions[action];
  }

  setPermissions(newPerms: Partial<CashierPermissions>) {
    this.permissions = { ...this.permissions, ...newPerms };
  }
}

export const permissionsStore = new PermissionsStore();
