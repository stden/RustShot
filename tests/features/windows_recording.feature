@windows @tray_start_record
Feature: Native Windows recording output
  Scenario: A live region recording becomes a finalized H.264 MP4
    When a small desktop region is recorded while an owned window changes
    Then stopping produces an MP4 with encoded media and a finalized movie index
