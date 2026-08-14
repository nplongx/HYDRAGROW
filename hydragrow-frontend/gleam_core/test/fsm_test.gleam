import fsm
import gleam/option.{None, Some}
import gleeunit/should

pub fn extract_fault_code_test() {
  // Test định dạng SystemFault:
  fsm.extract_fault_code("SystemFault:MAX_HOURLY_DOSE_EC")
  |> should.equal(Some("MAX_HOURLY_DOSE_EC"))

  // Test định dạng Fault:
  fsm.extract_fault_code("Fault:EC_DOSING_FAILED")
  |> should.equal(Some("EC_DOSING_FAILED"))

  // Test định dạng JSON lồng
  fsm.extract_fault_code("{\"Fault\":\"PH_DOSING_FAILED\"}")
  |> should.equal(Some("PH_DOSING_FAILED"))

  // Trạng thái bình thường -> Không có lỗi
  fsm.extract_fault_code("Monitoring")
  |> should.equal(None)
}

pub fn compute_health_test() {
  // Trạng thái Online + Score cao
  let health = fsm.compute_health(True, Some(95))
  health.score |> should.equal(95)
  health.label |> should.equal("Hoàn hảo")

  // Trạng thái Offline
  let offline_health = fsm.compute_health(False, Some(100))
  offline_health.score |> should.equal(0)
  offline_health.label |> should.equal("Mất kết nối")
}

pub fn friendly_state_test() {
  let state = fsm.friendly_state("MimoDosing", True)
  state.label |> should.equal("Đang bổ sung vi chất")
  state.tone |> should.equal("mist")
}
