# Ghi thẻ nạp firmware TOS v2 ra thẻ SD rồi đọc lại để kiểm tra SHA256. CẦN quyền Administrator.
# CHỐT AN TOÀN: chỉ ghi khi Disk khớp đúng số thứ tự + dung lượng + bus USB + không phải ổ hệ thống/boot.
# (Số serial của đầu đọc thẻ giống nhau cho mọi thẻ, nên KHÔNG đủ để phân biệt; dung lượng mới là chốt chính.)
param(
    [int]$DiskNumber = 2,
    [long]$Size = 125069950976,
    [string]$Image = 'C:\Users\X\dự án xây lại OS trimui\out\fw_v2\sd_recovery_tg4040_brickpro_ver1.1.1_20260717_tos2.img',
    [string]$ExpectSha = '07bea2c918b05575790483b5bd7fbb5124cbcea0064a8792be877e1c91c4dab7',
    [string]$Log = 'C:\Users\X\tools\ghi_the_sd_v2.log'
)
$ErrorActionPreference = 'Stop'
function L($m) { "$(Get-Date -Format HH:mm:ss) $m" | Add-Content -Path $Log -Encoding UTF8 }

Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
using Microsoft.Win32.SafeHandles;
public static class RawDisk {
    [DllImport("kernel32.dll", SetLastError = true, CharSet = CharSet.Unicode)]
    public static extern SafeFileHandle CreateFile(string name, uint access, uint share, IntPtr sec, uint disp, uint flags, IntPtr tmpl);
}
'@

try {
    L "=== bat dau"
    $isAdmin = ([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
    if (-not $isAdmin) { throw 'khong co quyen Administrator' }
    $d = Get-Disk -Number $DiskNumber
    if ($d.Size -ne $Size -or $d.BusType -ne 'USB' -or $d.IsSystem -or $d.IsBoot) {
        throw "Disk $DiskNumber KHONG khop the da xac nhan (size=$($d.Size) bus=$($d.BusType) system=$($d.IsSystem) boot=$($d.IsBoot)) -> dung, khong ghi"
    }
    $len = (Get-Item -LiteralPath $Image).Length
    if ($len % 512) { throw 'kich thuoc anh khong chia het 512' }
    # kiểm tra lại ảnh đúng bản đã kiểm thử trước khi ghi
    $h0 = (Get-FileHash -LiteralPath $Image -Algorithm SHA256).Hash.ToLower()
    if ($h0 -ne $ExpectSha) { throw "anh KHONG dung ban da kiem thu (sha256 $h0)" }
    L "the khop: $($d.FriendlyName) size=$Size. Anh dung ($len byte). Xoa phan vung..."
    if ($d.PartitionStyle -ne 'RAW') { Clear-Disk -Number $DiskNumber -RemoveData -RemoveOEM -Confirm:$false }
    Start-Sleep -Seconds 2

    $h = [RawDisk]::CreateFile("\\.\PhysicalDrive$DiskNumber", [uint32]3221225472, [uint32]3, [IntPtr]::Zero, [uint32]3, [uint32]0, [IntPtr]::Zero)
    if ($h.IsInvalid) { throw "khong mo duoc PhysicalDrive$DiskNumber (loi $([Runtime.InteropServices.Marshal]::GetLastWin32Error()))" }
    $disk = New-Object IO.FileStream($h, [IO.FileAccess]::ReadWrite, 4096)
    $src = [IO.File]::OpenRead($Image)
    $buf = New-Object byte[] (4MB)
    $done = 0L; $t0 = Get-Date
    while (($n = $src.Read($buf, 0, $buf.Length)) -gt 0) {
        $disk.Write($buf, 0, $n); $done += $n
        if (($done % 256MB) -eq 0) { L ("ghi {0}/{1} MB" -f ($done / 1MB), ($len / 1MB)) }
    }
    $disk.Flush(); $src.Close()
    L ("ghi xong {0} MB trong {1:N0}s. Doc lai de kiem tra..." -f ($done / 1MB), ((Get-Date) - $t0).TotalSeconds)

    $disk.Position = 0
    $sha = [Security.Cryptography.SHA256]::Create()
    [long]$left = $len
    while ($left -gt 0) {
        [int]$want = if ($left -lt $buf.Length) { [int]$left } else { $buf.Length }
        $n = $disk.Read($buf, 0, $want)
        if ($n -le 0) { throw 'doc lai bi ngat' }
        [void]$sha.TransformBlock($buf, 0, $n, $null, 0); $left -= $n
        if ((($len - $left) % 512MB) -eq 0) { L ("doc lai {0} MB" -f (($len - $left) / 1MB)) }
    }
    [void]$sha.TransformFinalBlock($buf, 0, 0)
    $disk.Close()
    $got = -join ($sha.Hash | ForEach-Object { $_.ToString('x2') })
    if ($got -eq $ExpectSha) { L "KIEM TRA OK: sha256 the = anh ($got)"; L 'XONG' }
    else { L "LOI: sha256 the $got khac anh $ExpectSha"; L 'THAT BAI' }
    Update-Disk -Number $DiskNumber -ErrorAction SilentlyContinue
} catch {
    L "LOI: $($_.Exception.Message)"
    L 'THAT BAI'
}
